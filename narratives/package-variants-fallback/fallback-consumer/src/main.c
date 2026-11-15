#include <stdio.h>
#include "codec.h"

int main(void) {
    printf("encoded size: %d\n", codec_encoded_size(12));
    return 0;
}
