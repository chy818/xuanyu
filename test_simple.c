#include <stdio.h>

void print_test() {
    printf("OK\n");
    fflush(stdout);
}

int main(int argc, char** argv) {
    print_test();
    return 0;
}