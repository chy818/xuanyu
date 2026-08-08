#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <time.h>

int64_t rt_utf8_codepoint_at(void* str, int64_t byte_pos) {
    if (!str) return -1;
    unsigned char* s = (unsigned char*)str;
    unsigned char c = s[byte_pos];
    if (c == 0) return -1;
    int64_t cp;
    if ((c & 0x80) == 0) { cp = c; }
    else if ((c & 0xE0) == 0xC0) { if (s[byte_pos+1]==0) return c; cp = ((c&0x1F)<<6)|(s[byte_pos+1]&0x3F); }
    else if ((c & 0xF0) == 0xE0) { if (s[byte_pos+1]==0||s[byte_pos+2]==0) return c; cp = ((c&0x0F)<<12)|((s[byte_pos+1]&0x3F)<<6)|(s[byte_pos+2]&0x3F); }
    else if ((c & 0xF8) == 0xF0) { if (s[byte_pos+1]==0||s[byte_pos+2]==0||s[byte_pos+3]==0) return c; cp = ((c&0x07)<<18)|((s[byte_pos+1]&0x3F)<<12)|((s[byte_pos+2]&0x3F)<<6)|(s[byte_pos+3]&0x3F); }
    else { cp = c; }
    return cp;
}

int64_t rt_utf8_byte_length(int64_t ch) {
    if (ch > 255) { if (ch < 0x800) return 2; if (ch < 0x10000) return 3; return 4; }
    unsigned char c = (unsigned char)ch;
    if ((c&0x80)==0) return 1; if ((c&0xE0)==0xC0) return 2; if ((c&0xF0)==0xE0) return 3; if ((c&0xF8)==0xF0) return 4;
    return 1;
}

// Simulate the XY lexer loop for skipping whitespace
int main() {
    // Read the test file
    FILE* f = fopen("test_gap3000.xy", "rb");
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    char* src = malloc(sz+1);
    fread(src, 1, sz, f); src[sz]=0;
    fclose(f);

    int64_t pos = 0, len = sz, line=1, col=1, cur_char;
    cur_char = rt_utf8_codepoint_at(src, 0);
    
    clock_t start = clock();
    
    // Simulate skipping 3000 spaces
    int count = 0;
    while (cur_char == 32 && pos < len) {
        int64_t advance = rt_utf8_byte_length(cur_char);
        pos += advance;
        col++;
        if (pos < len) cur_char = rt_utf8_codepoint_at(src, pos);
        else cur_char = 0;
        if (cur_char == 10) { line++; col=1; }
        count++;
    }
    
    clock_t end = clock();
    printf("Skipped %d spaces in %.3f ms\n", count, (double)(end-start)*1000/CLOCKS_PER_SEC);
    printf("Final pos=%lld, cur_char=%lld\n", pos, cur_char);
    
    free(src);
    return 0;
}
