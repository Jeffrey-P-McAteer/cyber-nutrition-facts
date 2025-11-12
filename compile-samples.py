
import os
import sys
import subprocess
import shutil
import traceback

import shlex
from pathlib import Path

vswhere = None
if os.name == 'nt':
    pip_packages_folder = r'C:\Workspace\pip-venv-01'
    os.makedirs(pip_packages_folder, exist_ok=True)
    sys.path.append(pip_packages_folder)
    try:
        import vswhere
    except:
        subprocess.run([sys.executable, '-m', 'pip', 'install', f'--target={pip_packages_folder}', 'vswhere'], check=False)
        import vswhere

def locate_latest_vs() -> Path:
    """
    Return the folder that contains the newest installation of Visual
    Studio (or the Build Tools).  The returned path is the root folder
    that contains ``VC\\Auxiliary\\Build\\vcvarsall.bat``.
    """
    # The vswhere executable is bundled with the package – it looks in the
    # registry and in the standard VS installation folders.
    # vw = vswhere.VSWhere()
    # Ask for the *latest* product that provides a C++ toolset
    result = vswhere.get_latest(
        requires="Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        # We want the full installation path (not just the components)
        # return_property="installationPath",
    )
    #print(f'result = {result}')
    result = result.get('installationPath', None)
    if not result:
        raise RuntimeError(
            "Could not locate a Visual Studio installation that contains the "
            "C++ build tools.  Make sure Visual Studio (or the Build Tools) "
            "are installed."
        )
    return Path(result)


def vcvarsall_path(vs_root: Path) -> Path:
    """
    Return the full path to the vcvarsall.bat script for the given VS root.
    """
    candidate = vs_root / "VC" / "Auxiliary" / "Build" / "vcvarsall.bat"
    if not candidate.is_file():
        raise FileNotFoundError(f"vcvarsall.bat not found at {candidate}")
    return candidate


# ----------------------------------------------------------------------
# 2️⃣ Helper: run vcvarsall.bat and capture the environment it creates
# ----------------------------------------------------------------------
def capture_vcvars(
    vcvars_path: Path, arch: str = "x64", extra_args: str = ""
) -> dict:
    """
    Execute ``vcvarsall.bat <arch> <extra_args>`` inside a temporary cmd.exe
    and return a dictionary with the environment that the batch file left
    behind.

    Parameters
    ----------
    vcvars_path: Path
        Full path to the vcvarsall.bat file.
    arch: str
        Target architecture (e.g. "x86", "x64", "x86_amd64", "x64_arm64" …)
    extra_args: str
        Anything you would normally pass after the architecture (rarely needed).

    Returns
    -------
    dict
        Environment mapping ready to be passed to subprocess.Popen(..., env=env).
    """
    # Build a tiny one‑liner that runs vcvarsall.bat and then prints the env
    # with the built‑in `set` command.  We need to make sure we use `cmd /s /c`
    # because the batch file may contain parentheses.
    cmd = (
        f'cmd /s /c "'
        f'"{vcvars_path}" {arch} {extra_args} && '
        f'set"'
    )
    # Run it and capture stdout (which contains NAME=VALUE lines)
    completed = subprocess.run(
        cmd,
        shell=True,
        capture_output=True,
        text=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"Running vcvarsall.bat failed (rc={completed.returncode})\n"
            f"stdout:{completed.stdout}\nstderr:{completed.stderr}"
        )
    # Parse the output into a dict
    env = {}
    for line in completed.stdout.splitlines():
        if "=" not in line:
            continue
        key, val = line.split("=", 1)
        env[key.upper()] = val
    # Preserve the original OS environment for things that vcvarsall does not set
    # (e.g. SystemRoot, TEMP, etc.)
    env.update(os.environ)
    return env


REPO_DIR = os.path.dirname(__file__)

def repo_path(*parts):
    return os.path.join(REPO_DIR, *list([x for x in parts if x is not None]))

def sample_c_file(name):
    return repo_path('samples', f'{name}.c')

def host_sample_exe_file(name):
    if os.name == 'nt':
        return repo_path('samples', f'{name}.exe')
    else:
        return repo_path('samples', f'{name}')

def target_sample_exe_file(name, target):
    if 'windows' in target:
        return repo_path('samples', f'{name}.exe')
    else:
        return repo_path('samples', f'{name}')

def host_compile_outfile_args(out_file_path):
    if os.name == 'nt':
        return [f'/Fe:', out_file_path]
    else:
        return [f'-o', out_file_path, '-g']

def compile_with_host_toolchain(sample_name):
    compilers = [
        'clang', 'gcc', 'cl'
    ]
    our_compiler = next(shutil.which(c) for c in compilers if shutil.which(c))
    cmd = [
        our_compiler,
        sample_c_file(sample_name),
        *host_compile_outfile_args(host_sample_exe_file(sample_name))
    ]
    print(f'> {" ".join(cmd)}')
    subprocess.run(cmd, check=True)

def cross_compile_if_zig_available(sample_name, zig_target):
    zig_exe = shutil.which('zig')
    if zig_exe:
        cmd = [
            'zig', 'cc', '-target', zig_target,
            sample_c_file(sample_name),
            '-o', target_sample_exe_file(sample_name, zig_target)
        ]
        print(f'> {" ".join(cmd)}')
        subprocess.run(cmd, check=True)
    else:
        print(f'WARNING: zig not found, skipping cross-compile for the target {zig_target}')

def main():
    if os.name == 'nt':
        # Visual Studio Complex Magic
        for name, value in capture_vcvars(vcvarsall_path(locate_latest_vs())).items():
            os.environ[name] = value


    sample_names = [
        'c_safe_a',
        'c_unsafe_a',
    ]

    for sample_name in sample_names:
        compile_with_host_toolchain(sample_name)
        if 'cross' in sys.argv:
            cross_compile_if_zig_available(sample_name, 'x86_64-windows-gnu')
            cross_compile_if_zig_available(sample_name, 'x86_64-linux')



if __name__ == '__main__':
    main()





