#include <stdio.h>
#include <stdlib.h>
#include <time.h>
#include <math.h>
#include <process.h>
#include "random.h"

void init() {
    unsigned int seed = (unsigned) time(NULL);
    seed ^= _getpid();
    seed ^= (unsigned int)clock();

    srand(seed);
}

int generateRandomNumber(int min, int max) {
    return min + rand() % (max - min + 1);
}

int generateRandomNumberExpo(int min, int max, float lambda) {
    float u = 0.0f;

    do {
        u = (float)rand() / RAND_MAX;
    } while(u == 0.0f);

    float x = -log(u) / lambda;

    int result = (int)x + min;

    if (result > max) {
        result = max;
    }

    return result;
}