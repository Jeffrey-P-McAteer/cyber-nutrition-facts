use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use capstone::arch::x86::{ArchMode as X86ArchMode, X86Operand, X86OperandType};
use capstone::{prelude::*, Capstone};
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};

/// Print a function call tree beginning at `entry_symbol` (e.g. "_start" or "main").
/// Uses `crate::DynResult` for error handling (should be `Result<T, Box<dyn Error>>`).
pub fn print_tree_of_elf<P: AsRef<Path>>(elf_path: P, entry_symbol: &str) -> crate::DynResult<()> {
    // Read file
    let data = fs::read(&elf_path)?;
    let obj = object::File::parse(&*data)?;

    // Collect executable/text sections for VA -> bytes mapping
    let mut sections: Vec<(u64, Vec<u8>, u64)> = Vec::new();
    for sec in obj.sections() {
        if sec.kind() == object::SectionKind::Text {
            if let Ok(bytes) = sec.data() {
                sections.push((sec.address(), bytes.to_vec(), sec.size()));
            }
        }
    }

    // Build symbol maps (text symbols)
    let mut addr_to_name: HashMap<u64, String> = HashMap::new();
    let mut name_to_addr: HashMap<String, u64> = HashMap::new();
    for sym in obj.symbols().chain(obj.dynamic_symbols()) {
        if sym.kind() == SymbolKind::Text {
            if let Ok(name) = sym.name() {
                let addr = sym.address();
                if addr != 0 {
                    addr_to_name.insert(addr, name.to_string());
                    name_to_addr.insert(name.to_string(), addr);
                }
            }
        }
    }

    // Choose entry address: prefer provided symbol, else ELF entry.
    let entry_addr = name_to_addr
        .get(entry_symbol)
        .copied()
        .unwrap_or_else(|| obj.entry());

    // Build Capstone using the documented builder API
    // (Capstone::new().x86().mode(...).detail(true).build())
    let cs = Capstone::new()
        .x86()
        .mode(X86ArchMode::Mode64)
        .detail(true)
        .build()
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?; // adapt to crate::DynResult

    // Closure to read bytes at a virtual address from collected sections
    let read_bytes = |addr: u64, size: usize| -> Option<&[u8]> {
        for (sec_addr, bytes, sec_size) in &sections {
            if addr >= *sec_addr && addr + (size as u64) <= *sec_addr + *sec_size {
                let off = (addr - *sec_addr) as usize;
                return Some(&bytes[off..off + size]);
            }
        }
        None
    };

    // visited set to avoid infinite recursion
    let mut visited: HashSet<u64> = HashSet::new();

    // Start DFS print
    dfs_print(&cs, entry_addr, 0, &read_bytes, &addr_to_name, &mut visited)?;

    Ok(())
}

/// Recursively disassemble from `addr`, print name (if any), and recurse into direct call targets.
/// - `read_bytes` should return Some(&[u8]) for bytes starting at VA `addr`.
fn dfs_print<'a>(
    cs: &Capstone,
    addr: u64,
    depth: usize,
    read_bytes: &impl Fn(u64, usize) -> Option<&'a [u8]>,
    addr_to_name: &HashMap<u64, String>,
    visited: &mut HashSet<u64>,
) -> crate::DynResult<()> {
    // indentation
    for _ in 0..depth {
        print!("    ");
    }
    if let Some(name) = addr_to_name.get(&addr) {
        println!("{} (0x{:x})", name, addr);
    } else {
        println!("0x{:x}", addr);
    }

    if !visited.insert(addr) {
        for _ in 0..(depth + 1) {
            print!("    ");
        }
        println!("(already visited)");
        return Ok(());
    }

    // Heuristic read length per function (adjust if needed)
    const READ_SZ: usize = 4096;
    let bytes = match read_bytes(addr, READ_SZ) {
        Some(b) => b,
        None => return Ok(()), // can't read bytes at this VA
    };

    // Disassemble chunk
    let insns = cs.disasm_all(bytes, addr)
        .map_err(|e| -> Box<dyn std::error::Error> { Box::new(e) })?;

    for insn in insns.iter() {
        // Use mnemonic approach: 'call' instructions. This avoids unstable numeric group constants.
        if let Some(mn) = insn.mnemonic() {
            if mn == "call" {
                if let Some(target) = extract_call_imm_target(cs, &insn) {
                    // recurse into direct immediate target
                    dfs_print(cs, target, depth + 1, read_bytes, addr_to_name, visited)?;
                } else {
                    // indirect call (call rax / call [rip+...]) — try to resolve RIP+disp -> pointer in section
                    if let Some(mem_target) = resolve_rip_relative_call(cs, &insn, read_bytes) {
                        dfs_print(cs, mem_target, depth + 1, read_bytes, addr_to_name, visited)?;
                    } else {
                        // couldn't resolve statically; print placeholder
                        for _ in 0..(depth + 1) {
                            print!("    ");
                        }
                        println!("(indirect call at 0x{:x})", insn.address());
                    }
                }
            } else if mn == "ret" {
                // stop at ret to avoid falling through to next function bytes
                break;
            }
        }
    }

    Ok(())
}

/// Try to extract an immediate call target from operand imm if present.
/// Returns Some(target_addr) on success.
fn extract_call_imm_target(cs: &Capstone, insn: &capstone::Insn) -> Option<u64> {
    let detail = cs.insn_detail(insn).ok()?;
    let arch_detail = detail.arch_detail();
    for op in arch_detail.operands() {
        // ArchOperand enum — match the X86 operand variant and inspect its op_type()
        match op {
            capstone::arch::ArchOperand::X86Operand(x86op) => {
                if let X86OperandType::Imm(imm) = x86op.op_type {
                    return Some(imm as u64);
                }
            }
            _ => {}
        }
    }
    None
}

/// Resolve common RIP-relative GOT/PLT style call: call [rip + disp] -> load pointer from that address if bytes are readable.
fn resolve_rip_relative_call<'a>(
    cs: &Capstone,
    insn: &capstone::Insn,
    read_bytes: &impl Fn(u64, usize) -> Option<&'a [u8]>,
) -> Option<u64> {
    // Inspect operands to find MEM with base RIP and displacement
    let detail = cs.insn_detail(insn).ok()?;
    let arch_detail = detail.arch_detail();
    for op in arch_detail.operands() {
        if let capstone::arch::ArchOperand::X86Operand(x86op) = op {
            if let X86OperandType::Mem(mem) = x86op.op_type {
                // mem.base is a register id: if it's RIP (X86_REG_RIP) then compute pointer address
                // mem.disp is i64 displacement
                // X86_REG_RIP constant is available via capstone::arch::x86::X86Reg::X86_REG_RIP
                use capstone::arch::x86::X86Reg;
                if mem.base() == capstone::RegId(X86Reg::X86_REG_RIP as u16) {
                    // address of the memory operand = insn.address() + insn.size() as u64 + disp
                    let addr_ptr = insn.address().wrapping_add(insn.bytes().len() as u64).wrapping_add(mem.disp() as u64);
                    // read 8 bytes for a 64-bit pointer
                    if let Some(ptr_bytes) = read_bytes(addr_ptr, 8) {
                        let mut arr = [0u8; 8];
                        arr.copy_from_slice(&ptr_bytes[0..8]);
                        let pointee = u64::from_le_bytes(arr);
                        return Some(pointee);
                    }
                }
            }
        }
    }
    None
}
