// Prints Well512 reference outputs for the Rust unit test.
#include "RandomNumberGenerator.h"
#include <cstdio>

int main() {
    unsigned seeds[] = {0, 12345};
    for (unsigned seed : seeds) {
        RandomNumberGenerator::generator().reseed(seed);
        printf("seed %u int:", seed);
        for (int i = 0; i < 20; i++) printf(" %d", RANDOM_INT(1000));
        printf("\n");
        printf("seed %u mixed:", seed);
        for (int i = 0; i < 10; i++) printf(" %d", RandomNumberGenerator::generator().getInteger(-7, 13));
        for (int i = 0; i < 10; i++) printf(" %d", RANDOM_BOOL() ? 1 : 0);
        for (int i = 0; i < 10; i++) printf(" %d", RANDOM_INT(2147483647));
        printf("\n");
    }
}
