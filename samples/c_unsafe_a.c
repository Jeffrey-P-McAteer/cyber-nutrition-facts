
#include <stdio.h>

void do_some_stuff(char* name) {
    printf("Hello, %s!\n", name);
}

int main(int argc, char** argv) {
    
    printf("Hello, Unsafe %s!\n", argv[1]); // OOB read if ARG1 not passed

    do_some_stuff(argv[2]);

    return 0;
}
