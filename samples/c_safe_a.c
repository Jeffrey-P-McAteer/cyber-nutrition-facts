
#include <stdio.h>

void do_some_stuff(char* name, int length) {
    printf("Hello, %.*s!\n", length, name);
}

int main(int argc, char** argv) {

    printf("Hello, Safe C!\n");

    char* name = "Test";
    do_some_stuff(name, 4);

    return 0;
}



