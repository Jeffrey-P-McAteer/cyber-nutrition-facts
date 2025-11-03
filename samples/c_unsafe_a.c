
#include <stdio.h>

int main(int argc, char** argv) {
    
    printf("Hello, Unsafe %s!\n", argv[1]); // OOB read if ARG1 not passed

    return 0;
}
