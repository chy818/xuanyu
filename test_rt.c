#include <stdio.h>
#include <string.h>
#include <stdint.h>

int64_t rt_utf8_codepoint_at(void* str, int64_t byte_pos) {
    if (!str) return -1;
    char* s = (char*)str;
    int64_t len = (int64_t)strlen(s);
    if (byte_pos < 0 || byte_pos >= len) return -1;
    unsigned char c = (unsigned char)s[byte_pos];
    int64_t cp;
    if ((c & 0x80) == 0) { cp = c; }
    else if ((c & 0xE0) == 0xC0 && byte_pos + 1 < len) {
        cp = ((c & 0x1F) << 6) | ((unsigned char)s[byte_pos + 1] & 0x3F);
    } else if ((c & 0xF0) == 0xE0 && byte_pos + 2 < len) {
        cp = ((c & 0x0F) << 12) | (((unsigned char)s[byte_pos + 1] & 0x3F) << 6) | ((unsigned char)s[byte_pos + 2] & 0x3F);
    } else if ((c & 0xF8) == 0xF0 && byte_pos + 3 < len) {
        cp = ((c & 0x07) << 18) | (((unsigned char)s[byte_pos + 1] & 0x3F) << 12) | (((unsigned char)s[byte_pos + 2] & 0x3F) << 6) | ((unsigned char)s[byte_pos + 3] & 0x3F);
    } else { cp = c; }
    return cp;
}

int main() {
    char* s = "函数 主";
    printf("Position 0: %lld (expected 20989 函)\n", rt_utf8_codepoint_at(s, 0));
    printf("Position 3: %lld (expected 25968 数)\n", rt_utf8_codepoint_at(s, 3));
    printf("Position 6: %lld (expected 32 space)\n", rt_utf8_codepoint_at(s, 6));
    printf("Position 7: %lld (expected 20027 主)\n", rt_utf8_codepoint_at(s, 7));
    // 50K spaces test
    char* big = malloc(50001);
    memset(big, ' ', 50000);
    big[50000] = 0;
    for (int i = 0; i < 50000; i++) {
        int64_t cp = rt_utf8_codepoint_at(big, i);
        if (cp != 32) { printf("FAIL at %d: %lld\n", i, cp); return 1; }
    }
    printf("50K spaces: PASS\n");
    free(big);
    return 0;
}
