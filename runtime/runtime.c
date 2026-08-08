/* 抑制 MSVC 弃用警告 */
#define _CRT_SECURE_NO_WARNINGS
#define _CRT_NONSTDC_NO_DEPRECATE

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <time.h>

#ifdef _WIN32
#include <windows.h>
#include <process.h>
#define popen _popen
#define pclose _pclose
#endif

/* ============ Arena 内存池 (bump allocator) ============ */
typedef struct {
    char* buffer;
    int64_t capacity;
    int64_t offset;
} Arena;

static Arena* g_arena = NULL;  /* 全局Arena，编译阶段设置 */

Arena* arena_new(int64_t initial_capacity) {
    Arena* a = (Arena*)malloc(sizeof(Arena));
    if (!a) return NULL;
    a->buffer = (char*)malloc(initial_capacity);
    a->capacity = a->buffer ? initial_capacity : 0;
    a->offset = 0;
    return a;
}

void arena_reset(Arena* a) {
    if (a) a->offset = 0;
}

void arena_free_all(Arena* a) {
    if (a) {
        free(a->buffer);
        free(a);
    }
}

/* 线程级Arena访问 */
void rt_arena_set(void* arena) { g_arena = (Arena*)arena; }
void* rt_arena_get() { return g_arena; }

/* 从Arena分配（bump），Arena满时自动扩容 */
void* arena_alloc(Arena* a, int64_t size) {
    if (!a || size <= 0) return NULL;
    /* 8字节对齐 */
    int64_t aligned = (size + 7) & ~((int64_t)7);
    if (a->offset + aligned > a->capacity) {
        /* 扩容：double + 请求大小 */
        int64_t new_cap = a->capacity * 2;
        if (new_cap < a->offset + aligned + 65536)
            new_cap = a->offset + aligned + 65536;
        char* new_buf = (char*)realloc(a->buffer, new_cap);
        if (!new_buf) return NULL;
        a->buffer = new_buf;
        a->capacity = new_cap;
    }
    void* ptr = a->buffer + a->offset;
    a->offset += aligned;
    return ptr;
}

/* Arena版本的malloc — 自动使用全局Arena，首次调用时自动创建 */
void* rt_arena_malloc(int64_t size) {
    if (!g_arena) {
        /* 懒初始化：首次分配时自动创建 2MB Arena */
        g_arena = arena_new(2 * 1024 * 1024);
    }
    if (g_arena) {
        void* ptr = arena_alloc(g_arena, size);
        if (ptr) return ptr;
        /* Arena满了，回退到系统malloc */
    }
    return malloc(size);
}

/* Arena版本的strdup */
char* rt_arena_strdup(const char* s) {
    if (!s) return NULL;
    int64_t len = (int64_t)strlen(s) + 1;
    char* buf = (char*)rt_arena_malloc(len);
    if (buf) memcpy(buf, s, len);
    return buf;
}

/** List structure */
typedef struct {
    void** items;
    int64_t count;
    int64_t capacity;
} List;

/* Create new list */
void* rt_list_new() {
    List* list = (List*)rt_arena_malloc(sizeof(List));
    if (!list) return NULL;
    list->items = NULL;
    list->count = 0;
    list->capacity = 0;
    return list;
}

/* Alias for rt_list_new (used by codegen) */
void* create_list() {
    return rt_list_new();
}

/* Closure structure - matches LLVM IR layout
 * Layout: [func_ptr: 8 bytes][captured_count: 8 bytes][captured_vars...][param_slots...]
 */
typedef struct {
    void* func_ptr;
    int64_t captured_count;
    /* captured variables and param slots follow */
} Closure;

/* Free closure memory */
void rt_closure_destroy(void* closure_ptr) {
    if (!closure_ptr) return;
    free(closure_ptr);
}

/* Append to list (arena-friendly: 扩容时 arena分配 + 拷贝，避免 realloc) */
void rt_list_append(void* list_ptr, void* item) {
    if (!list_ptr) return;
    List* list = (List*)list_ptr;
    if (list->count >= list->capacity) {
        int64_t new_cap = list->capacity == 0 ? 16 : list->capacity * 2;
        void** new_items = (void**)rt_arena_malloc(new_cap * sizeof(void*));
        if (!new_items) return;
        if (list->items && list->count > 0) {
            memcpy(new_items, list->items, list->count * sizeof(void*));
        }
        list->items = new_items;
        list->capacity = new_cap;
    }
    list->items[list->count++] = item;
}

/* Alias for rt_list_append (used by codegen) */
void list_add(void* list_ptr, void* item) {
    rt_list_append(list_ptr, item);
}

/* Get from list */
void* rt_list_get(void* list_ptr, int64_t index) {
    if (!list_ptr) return NULL;
    List* list = (List*)list_ptr;
    if (index < 0 || index >= list->count) return NULL;
    return list->items[index];
}

/* List length */
int64_t rt_list_len(void* list_ptr) {
    if (!list_ptr) return 0;
    List* list = (List*)list_ptr;
    return list->count;
}

/* Set list element at index */
void rt_list_set(void* list_ptr, int64_t index, void* value) {
    if (!list_ptr) return;
    List* list = (List*)list_ptr;
    if (index < 0 || index >= list->count) return;
    list->items[index] = value;
}

/* Print functions */
void print(void* str) {
    if (str) printf("%s", (char*)str);
    fflush(stdout);
}

void print_int(int64_t val) {
    printf("%lld", val);
    fflush(stdout);
}

void print_float(double val) {
    printf("%g", val);
}

void print_bool(int val) {
    printf("%s", val ? "true" : "false");
}

/* rt_ prefix aliases for LLVM IR compatibility */
void rt_print(void* str) {
    print(str);
    fflush(stdout);
}

void rt_println(void* str) {
    print(str);
    printf("\n");
    fflush(stdout);
}

void rt_print_int(int64_t val) {
    print_int(val);
}

void rt_print_float(double val) {
    print_float(val);
}

/* Type conversion functions */
void* rt_int_to_str(int64_t val) {
    char buffer[32];
    snprintf(buffer, sizeof(buffer), "%lld", val);
    return strdup(buffer);
}

int64_t rt_str_to_int(void* str) {
    if (!str) return 0;
    return strtoll((char*)str, NULL, 10);
}

void* rt_float_to_str(double val) {
    char buffer[64];
    snprintf(buffer, sizeof(buffer), "%g", val);
    return strdup(buffer);
}

double rt_str_to_double(void* str) {
    if (!str) return 0.0;
    return strtod((char*)str, NULL);
}

/* String functions */
void* rt_str_new(const char* utf8_content) {
    if (!utf8_content) return NULL;
    return strdup(utf8_content);
}

void* rt_str_concat(void* a, void* b) {
    if (!a || !b) return NULL;
    size_t len_a = strlen((char*)a);
    size_t len_b = strlen((char*)b);
    char* result = (char*)malloc(len_a + len_b + 1);
    if (!result) return NULL;
    strcpy(result, (char*)a);
    strcat(result, (char*)b);
    return result;
}

/* 字符串比较函数 */
int64_t rt_str_eq(void* a, void* b) {
    if (!a && !b) return 1;  // 两个都是 NULL，认为相等
    if (!a || !b) return 0;  // 一个是 NULL，另一个不是，不相等
    return strcmp((char*)a, (char*)b) == 0 ? 1 : 0;
}

int64_t rt_str_ne(void* a, void* b) {
    return !rt_str_eq(a, b);
}

int64_t rt_str_lt(void* a, void* b) {
    if (!a || !b) return 0;
    return strcmp((char*)a, (char*)b) < 0 ? 1 : 0;
}

int64_t rt_str_le(void* a, void* b) {
    if (!a || !b) return 0;
    return strcmp((char*)a, (char*)b) <= 0 ? 1 : 0;
}

int64_t rt_str_gt(void* a, void* b) {
    if (!a || !b) return 0;
    return strcmp((char*)a, (char*)b) > 0 ? 1 : 0;
}

int64_t rt_str_ge(void* a, void* b) {
    if (!a || !b) return 0;
    return strcmp((char*)a, (char*)b) >= 0 ? 1 : 0;
}

/* Character classification functions */
int is_space(void* ch_ptr) {
    if (!ch_ptr) return 0;
    char ch = *((char*)ch_ptr);
    return (ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r') ? 1 : 0;
}

int is_digit(void* ch_ptr) {
    if (!ch_ptr) return 0;
    char ch = *((char*)ch_ptr);
    return (ch >= '0' && ch <= '9') ? 1 : 0;
}

int is_alpha(void* ch_ptr) {
    if (!ch_ptr) return 0;
    char ch = *((char*)ch_ptr);
    return ((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch == '_') ? 1 : 0;
}

int is_alnum(void* ch_ptr) {
    if (!ch_ptr) return 0;
    return (is_alpha(ch_ptr) || is_digit(ch_ptr)) ? 1 : 0;
}

/* Character to code conversion - expects a string pointer */
int64_t rt_char_to_code(void* ch_ptr) {
    if (!ch_ptr) return 0;
    unsigned char* s = (unsigned char*)ch_ptr;
    // UTF-8 解码为完整 Unicode 码点
    if ((s[0] & 0x80) == 0) {
        return (int64_t)s[0];  // ASCII
    } else if ((s[0] & 0xE0) == 0xC0) {
        return (int64_t)(((s[0] & 0x1F) << 6) | (s[1] & 0x3F));
    } else if ((s[0] & 0xF0) == 0xE0) {
        return (int64_t)(((s[0] & 0x0F) << 12) | ((s[1] & 0x3F) << 6) | (s[2] & 0x3F));
    } else if ((s[0] & 0xF8) == 0xF0) {
        return (int64_t)(((s[0] & 0x07) << 18) | ((s[1] & 0x3F) << 12) | ((s[2] & 0x3F) << 6) | (s[3] & 0x3F));
    }
    return (int64_t)s[0];
}

/* Code to character conversion */
void* rt_code_to_char(int64_t code) {
    char* result = (char*)malloc(2);
    if (!result) return NULL;
    result[0] = (char)(code & 0xFF);
    result[1] = '\0';
    return result;
}

/* Error function */
void rt_error(void* msg) {
    if (msg) {
        fprintf(stderr, "Error: %s\n", (char*)msg);
    }
    exit(1);
}

/* List functions (without rt_ prefix for compatibility) */
int64_t list_len(void* list_ptr) {
    return rt_list_len(list_ptr);
}

void* list_get(void* list_ptr, int64_t index) {
    return rt_list_get(list_ptr, index);
}

/* String functions */
int64_t rt_string_len(void* str) {
    if (!str) return 0;
    return strlen((char*)str);
}

/* UTF-8 helper functions */
int64_t rt_utf8_byte_length(int64_t ch) {
    // 如果值是 Unicode 码点（> 255），按码点范围计算 UTF-8 字节数
    if (ch > 255) {
        if (ch < 0x800) return 2;
        if (ch < 0x10000) return 3;
        return 4;
    }
    // 否则按首字节判断
    unsigned char c = (unsigned char)ch;
    if ((c & 0x80) == 0) return 1;
    if ((c & 0xE0) == 0xC0) return 2;
    if ((c & 0xF0) == 0xE0) return 3;
    if ((c & 0xF8) == 0xF0) return 4;
    return 1;
}

int64_t rt_is_utf8_leader(int64_t ch) {
    if (ch > 255) return 1;  // Unicode 码点始终是有效的字符起始
    unsigned char c = (unsigned char)ch;
    return ((c & 0x80) == 0) ||
           ((c & 0xE0) == 0xC0) ||
           ((c & 0xF0) == 0xE0) ||
           ((c & 0xF8) == 0xF0);
}

int64_t rt_is_utf8_continuation(int64_t ch) {
    unsigned char c = (unsigned char)ch;
    return (c & 0xC0) == 0x80;
}

void* rt_string_char_at(void* str, int64_t byte_index) {
    if (!str) return NULL;
    char* s = (char*)str;
    int64_t len = strlen(s);
    if (byte_index < 0 || byte_index >= len) return strdup("");

    // UTF-8 感知：返回完整的 Unicode 字符（1-4 字节）
    unsigned char c = (unsigned char)s[byte_index];
    int char_len = 1;
    if ((c & 0x80) == 0) {
        char_len = 1;
    } else if ((c & 0xE0) == 0xC0) {
        char_len = 2;
    } else if ((c & 0xF0) == 0xE0) {
        char_len = 3;
    } else if ((c & 0xF8) == 0xF0) {
        char_len = 4;
    }

    // 确保不超出字符串边界
    if (byte_index + char_len > len) char_len = (int)(len - byte_index);

    char* result = (char*)malloc(char_len + 1);
    if (!result) return NULL;
    memcpy(result, s + byte_index, char_len);
    result[char_len] = '\0';
    return result;
}

/* Fast UTF-8 codepoint decode at byte position — zero allocation, O(1) */
int64_t rt_utf8_codepoint_at(void* str, int64_t byte_pos) {
    if (!str) return -1;
    unsigned char* s = (unsigned char*)str;
    unsigned char c = s[byte_pos];
    if (c == 0) return -1;  /* End of string */

    int64_t cp;

    if ((c & 0x80) == 0) {
        cp = c;
    } else if ((c & 0xE0) == 0xC0) {
        if (s[byte_pos + 1] == 0) return c;
        cp = ((c & 0x1F) << 6) | (s[byte_pos + 1] & 0x3F);
    } else if ((c & 0xF0) == 0xE0) {
        if (s[byte_pos + 1] == 0 || s[byte_pos + 2] == 0) return c;
        cp = ((c & 0x0F) << 12) | ((s[byte_pos + 1] & 0x3F) << 6) | (s[byte_pos + 2] & 0x3F);
    } else if ((c & 0xF8) == 0xF0) {
        if (s[byte_pos + 1] == 0 || s[byte_pos + 2] == 0 || s[byte_pos + 3] == 0) return c;
        cp = ((c & 0x07) << 18) | ((s[byte_pos + 1] & 0x3F) << 12) | ((s[byte_pos + 2] & 0x3F) << 6) | (s[byte_pos + 3] & 0x3F);
    } else {
        cp = c;  /* Invalid UTF-8, return raw byte */
    }
    return cp;
}

/* Zero-strlen substring — caller guarantees start/end are within string bounds */
void* rt_substring_fast(void* str, int64_t start, int64_t end) {
    if (!str) return NULL;
    char* s = (char*)str;
    if (start < 0) start = 0;
    if (start >= end) return rt_arena_strdup("");
    int64_t len = end - start;
    char* result = (char*)rt_arena_malloc(len + 1);
    if (!result) return NULL;
    memcpy(result, s + start, len);
    result[len] = '\0';
    return result;
}

/* Zero-copy token comparison: compare source[start..end] against expected string */
int64_t rt_token_eq(void* source, int64_t start, int64_t end, void* expected) {
    if (!source || !expected) return 0;
    int64_t len = end - start;
    if (len != (int64_t)strlen((char*)expected)) return 0;
    return memcmp((char*)source + start, (char*)expected, len) == 0 ? 1 : 0;
}

void* str_concat(void* a, void* b) {
    if (!a || !b) return NULL;
    size_t len_a = strlen((char*)a);
    size_t len_b = strlen((char*)b);
    char* result = (char*)malloc(len_a + len_b + 1);
    if (!result) return NULL;
    strcpy(result, (char*)a);
    strcat(result, (char*)b);
    return result;
}

void* str_slice(void* str, int64_t start, int64_t end) {
    if (!str) return NULL;
    char* s = (char*)str;
    int64_t len = strlen(s);
    if (start < 0) start = 0;
    if (end > len) end = len;
    if (start >= end) return rt_arena_strdup("");
    char* result = (char*)rt_arena_malloc(end - start + 1);
    if (!result) return NULL;
    strncpy(result, s + start, end - start);
    result[end - start] = '\0';
    return result;
}

void* str_contains(void* str, void* substr) {
    if (!str || !substr) return NULL;
    return strstr((char*)str, (char*)substr) ? (void*)1 : NULL;
}

/* Integer to string */
void* int_to_str(int64_t val) {
    char* result = (char*)malloc(32);
    if (!result) return NULL;
    sprintf(result, "%lld", val);
    return result;
}

/* Integer to float */
double int_to_float(int64_t val) {
    return (double)val;
}

/* Float to integer */
int64_t float_to_int(double val) {
    return (int64_t)val;
}

/* String to integer */
int64_t str_to_int(void* str) {
    if (!str) return 0;
    return atoll((char*)str);
}

/* File functions */
void* file_read(void* path) {
    if (!path) return NULL;
    FILE* f = fopen((char*)path, "rb");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    char* result = (char*)malloc(size + 1);
    if (!result) { fclose(f); return NULL; }
    size_t read = fread(result, 1, size, f);
    result[read] = '\0';
    fclose(f);
    return result;
}

int32_t file_write(void* path, void* content) {
    if (!path || !content) return -1;
    FILE* f = fopen((char*)path, "w");
    if (!f) return -1;
    fprintf(f, "%s", (char*)content);
    fclose(f);
    return 0;
}

int32_t file_exists(void* path) {
    if (!path) return 0;
    FILE* f = fopen((char*)path, "r");
    if (f) { fclose(f); return 1; }
    return 0;
}

int32_t file_delete(void* path) {
    if (!path) return -1;
    return remove((char*)path);
}

/* Command execution */
int32_t exec_cmd(void* cmd) {
    if (!cmd) return -1;
    return system((char*)cmd);
}

void* cmd_output(void* cmd) {
    if (!cmd) return NULL;
    FILE* pipe = popen((char*)cmd, "r");
    if (!pipe) return NULL;
    char buffer[1024];
    char* result = strdup("");
    while (fgets(buffer, sizeof(buffer), pipe)) {
        char* new_result = (char*)malloc(strlen(result) + strlen(buffer) + 1);
        strcpy(new_result, result);
        strcat(new_result, buffer);
        free(result);
        result = new_result;
    }
    pclose(pipe);
    return result;
}

/* Command line arguments */
int64_t argc_val = 0;
char** argv_val = NULL;

void init_args(int argc, char** argv) {
    argc_val = argc;
    argv_val = argv;
}

int64_t argc() {
    return argc_val;
}

void* argv(int64_t index) {
    if (index < 0 || index >= argc_val) return NULL;
    return argv_val[index];
}

/* Input functions */
int64_t rt_input_int() {
    char buffer[64];
    if (fgets(buffer, sizeof(buffer), stdin) != NULL) {
        /* Remove trailing newline */
        size_t len = strlen(buffer);
        if (len > 0 && buffer[len-1] == '\n') buffer[len-1] = '\0';
        /* Skip UTF-8 BOM if present (0xEF 0xBB 0xBF) */
        unsigned char* p = (unsigned char*)buffer;
        if (p[0] == 0xEF && p[1] == 0xBB && p[2] == 0xBF) {
            p += 3;
        }
        /* Skip leading whitespace */
        while (*p == ' ' || *p == '\t') p++;
        if (*p == '\0') return 0;
        return atoll((char*)p);
    }
    return 0;
}

void* rt_input_text() {
    static char buffer[4096];
    if (fgets(buffer, sizeof(buffer), stdin)) {
        size_t len = strlen(buffer);
        if (len > 0 && buffer[len-1] == '\n') buffer[len-1] = '\0';
        return strdup(buffer);
    }
    return NULL;
}

/* Read line from stdin */
void* rt_readline() {
    return rt_input_text();
}

/* Memory management functions */
void* rt_malloc(int64_t size) {
    if (size <= 0) return NULL;
    return malloc((size_t)size);
}

void rt_free(void* ptr) {
    if (ptr) free(ptr);
}

void* rt_realloc(void* ptr, int64_t new_size) {
    if (new_size <= 0) return NULL;
    return realloc(ptr, (size_t)new_size);
}

/* Additional string functions */
void* rt_string_concat(void* a, void* b) {
    return str_concat(a, b);
}

void* rt_string_substring(void* str, int64_t start, int64_t end) {
    return str_slice(str, start, end);
}

int64_t rt_string_indexOf(void* str, void* substr) {
    if (!str || !substr) return -1;
    char* result = strstr((char*)str, (char*)substr);
    if (result) return result - (char*)str;
    return -1;
}

int64_t rt_string_lastIndexOf(void* str, void* substr) {
    if (!str || !substr) return -1;
    char* str_copy = strdup((char*)str);
    char* last_result = NULL;
    char* result = strstr(str_copy, (char*)substr);
    while (result) {
        last_result = result;
        result = strstr(result + 1, (char*)substr);
    }
    int64_t index = -1;
    if (last_result) {
        index = last_result - str_copy;
    }
    free(str_copy);
    return index;
}

void* rt_string_toUpperCase(void* str) {
    if (!str) return NULL;
    char* result = strdup((char*)str);
    for (char* p = result; *p; p++) {
        if (*p >= 'a' && *p <= 'z') {
            *p = *p - 'a' + 'A';
        }
    }
    return result;
}

void* rt_string_toLowerCase(void* str) {
    if (!str) return NULL;
    char* result = strdup((char*)str);
    for (char* p = result; *p; p++) {
        if (*p >= 'A' && *p <= 'Z') {
            *p = *p - 'A' + 'a';
        }
    }
    return result;
}

int64_t rt_string_compareTo(void* str1, void* str2) {
    if (!str1 && !str2) return 0;
    if (!str1) return -1;
    if (!str2) return 1;
    return strcmp((char*)str1, (char*)str2);
}

void* rt_string_trim(void* str) {
    if (!str) return NULL;
    char* s = (char*)str;
    while (*s == ' ' || *s == '\t' || *s == '\n' || *s == '\r') s++;
    char* end = s + strlen(s) - 1;
    while (end > s && (*end == ' ' || *end == '\t' || *end == '\n' || *end == '\r')) end--;
    int64_t len = end - s + 1;
    char* result = (char*)malloc(len + 1);
    strncpy(result, s, len);
    result[len] = '\0';
    return result;
}

void* rt_string_replace(void* str, void* old_substr, void* new_substr) {
    if (!str || !old_substr) return str ? strdup((char*)str) : NULL;
    if (!new_substr) new_substr = "";

    char* result = (char*)malloc(strlen((char*)str) * 2 + 1);
    result[0] = '\0';

    char* current = (char*)str;
    char* match = strstr(current, (char*)old_substr);
    size_t old_len = strlen((char*)old_substr);
    size_t new_len = strlen((char*)new_substr);

    while (match) {
        strncat(result, current, match - current);
        strcat(result, (char*)new_substr);
        current = match + old_len;
        match = strstr(current, (char*)old_substr);
    }
    strcat(result, current);

    return result;
}

void* rt_string_split(void* str, void* delimiter) {
    if (!str || !delimiter) return rt_list_new();

    List* result = (List*)rt_list_new();
    char* str_copy = strdup((char*)str);
    char* token = strtok(str_copy, (char*)delimiter);
    while (token) {
        rt_list_append(result, strdup(token));
        token = strtok(NULL, (char*)delimiter);
    }
    free(str_copy);
    return result;
}

void* rt_string_startsWith(void* str, void* prefix) {
    if (!str || !prefix) return NULL;
    size_t prefix_len = strlen((char*)prefix);
    if (strlen((char*)str) < prefix_len) return NULL;
    return strncmp((char*)str, (char*)prefix, prefix_len) == 0 ? (void*)1 : NULL;
}

void* rt_string_endsWith(void* str, void* suffix) {
    if (!str || !suffix) return NULL;
    size_t str_len = strlen((char*)str);
    size_t suffix_len = strlen((char*)suffix);
    if (str_len < suffix_len) return NULL;
    return strcmp((char*)str + str_len - suffix_len, (char*)suffix) == 0 ? (void*)1 : NULL;
}

int64_t rt_string_isEmpty(void* str) {
    if (!str) return 1;
    return strlen((char*)str) == 0 ? 1 : 0;
}

void* rt_string_fromChar(int64_t char_code) {
    char* result = (char*)malloc(8);
    int len = 0;
    if (char_code < 0x80) {
        result[0] = (char)char_code;
        len = 1;
    } else if (char_code < 0x800) {
        result[0] = (char)(0xC0 | (char_code >> 6));
        result[1] = (char)(0x80 | (char_code & 0x3F));
        len = 2;
    } else if (char_code < 0x10000) {
        result[0] = (char)(0xE0 | (char_code >> 12));
        result[1] = (char)(0x80 | ((char_code >> 6) & 0x3F));
        result[2] = (char)(0x80 | (char_code & 0x3F));
        len = 3;
    } else {
        result[0] = (char)(0xF0 | (char_code >> 18));
        result[1] = (char)(0x80 | ((char_code >> 12) & 0x3F));
        result[2] = (char)(0x80 | ((char_code >> 6) & 0x3F));
        result[3] = (char)(0x80 | (char_code & 0x3F));
        len = 4;
    }
    result[len] = '\0';
    return result;
}

/* Process control functions */
int64_t rt_exit(int64_t code) {
    exit((int)code);
    return code;
}

void rt_abort(void) {
    abort();
}

void rt_assert(int64_t condition, void* message) {
    if (!condition) {
        if (message) {
            fprintf(stderr, "Assertion failed: %s\n", (char*)message);
        } else {
            fprintf(stderr, "Assertion failed\n");
        }
        abort();
    }
}

/* Time functions */
int64_t rt_time(void) {
    return (int64_t)time(NULL);
}

int64_t rt_clock(void) {
    return (int64_t)clock();
}

/* Additional list functions */
int64_t rt_list_isEmpty(void* list_ptr) {
    if (!list_ptr) return 1;
    List* list = (List*)list_ptr;
    return list->count == 0 ? 1 : 0;
}

void rt_list_clear(void* list_ptr) {
    if (!list_ptr) return;
    List* list = (List*)list_ptr;
    list->count = 0;
}

void* rt_list_clone(void* list_ptr) {
    if (!list_ptr) return NULL;
    List* original = (List*)list_ptr;
    List* cloned = (List*)rt_list_new();
    for (int64_t i = 0; i < original->count; i++) {
        rt_list_append(cloned, original->items[i]);
    }
    return cloned;
}

int64_t rt_list_indexOf(void* list_ptr, void* item) {
    if (!list_ptr) return -1;
    List* list = (List*)list_ptr;
    for (int64_t i = 0; i < list->count; i++) {
        if (list->items[i] == item) return i;
    }
    return -1;
}

void rt_list_insert(void* list_ptr, int64_t index, void* item) {
    if (!list_ptr) return;
    List* list = (List*)list_ptr;
    if (index < 0 || index > list->count) return;
    if (list->count >= list->capacity) {
        int64_t new_cap = list->capacity == 0 ? 8 : list->capacity * 2;
        void** new_items = (void**)realloc(list->items, new_cap * sizeof(void*));
        if (!new_items) return;
        list->items = new_items;
        list->capacity = new_cap;
    }
    for (int64_t i = list->count; i > index; i--) {
        list->items[i] = list->items[i - 1];
    }
    list->items[index] = item;
    list->count++;
}

void rt_list_remove(void* list_ptr, int64_t index) {
    if (!list_ptr) return;
    List* list = (List*)list_ptr;
    if (index < 0 || index >= list->count) return;
    for (int64_t i = index; i < list->count - 1; i++) {
        list->items[i] = list->items[i + 1];
    }
    list->count--;
}

int64_t rt_list_contains(void* list_ptr, void* item) {
    if (!list_ptr) return 0;
    List* list = (List*)list_ptr;
    for (int64_t i = 0; i < list->count; i++) {
        if (list->items[i] == item) return 1;
    }
    return 0;
}

/* Hash function for strings */
int64_t rt_hash(void* str) {
    if (!str) return 0;
    int64_t hash = 0;
    char* s = (char*)str;
    while (*s) {
        hash = hash * 31 + *s;
        s++;
    }
    return hash;
}

/* Alias functions for L2 standard library */
void* rt_list_clone_impl(void* list_ptr) {
    return rt_list_clone(list_ptr);
}

void rt_list_clear_impl(void* list_ptr) {
    rt_list_clear(list_ptr);
}

int64_t rt_list_indexOf_impl(void* list_ptr, void* item) {
    return rt_list_indexOf(list_ptr, item);
}

void rt_list_insert_impl(void* list_ptr, int64_t index, void* item) {
    rt_list_insert(list_ptr, index, item);
}

void rt_list_remove_impl(void* list_ptr, int64_t index) {
    rt_list_remove(list_ptr, index);
}

int64_t rt_list_contains_impl(void* list_ptr, void* item) {
    return rt_list_contains(list_ptr, item);
}

int64_t rt_string_indexOf_impl(void* str, void* substr) {
    return rt_string_indexOf(str, substr);
}

int64_t rt_string_lastIndexOf_impl(void* str, void* substr) {
    return rt_string_lastIndexOf(str, substr);
}

void* rt_string_toUpperCase_impl(void* str) {
    return rt_string_toUpperCase(str);
}

void* rt_string_toLowerCase_impl(void* str) {
    return rt_string_toLowerCase(str);
}

int64_t rt_string_compareTo_impl(void* str1, void* str2) {
    return rt_string_compareTo(str1, str2);
}

void* rt_string_trim_impl(void* str) {
    return rt_string_trim(str);
}

void* rt_string_replace_impl(void* str, void* old_substr, void* new_substr) {
    return rt_string_replace(str, old_substr, new_substr);
}

void* rt_string_split_impl(void* str, void* delimiter) {
    return rt_string_split(str, delimiter);
}

void* rt_string_startsWith_impl(void* str, void* prefix) {
    return rt_string_startsWith(str, prefix);
}

void* rt_string_endsWith_impl(void* str, void* suffix) {
    return rt_string_endsWith(str, suffix);
}

int64_t rt_string_isEmpty_impl(void* str) {
    return rt_string_isEmpty(str);
}

void* rt_string_fromChar_impl(int64_t char_code) {
    return rt_string_fromChar(char_code);
}

void* rt_string_concat_impl(void* a, void* b) {
    return rt_string_concat(a, b);
}

void* rt_string_substring_impl(void* str, int64_t start, int64_t end) {
    return rt_string_substring(str, start, end);
}

int64_t rt_list_isEmpty_impl(void* list_ptr) {
    return rt_list_isEmpty(list_ptr);
}

/* ================================================================
 * 协程调度器（轻量状态机最小基元）
 * ----------------------------------------------------------------
 * 设计背景：v0.3.0 异步采用「轻量协程」模型：
 *   - 异步函数编译为状态机（state + resume），启动/等待 生成挂起/恢复
 *   - 本文件提供调度器的 C 基座：spawn / run / await / yield 原语
 * 最小可行性说明（prep 阶段）：
 *   - 协程以 函数指针 + 单个 i64 参数 + 返回值 的约定运行
 *   - 完整的状态机转换(codegen_s)属于 v0.3.0 正式实现，cauto在此打桩
 *   - 真实并发/挂起语义由后续 codegen 状态机 + 本调度器驱动
 * ================================================================ */

typedef struct {
    int64_t (*fn)(int64_t);  /* 协程入口函数 */
    int64_t arg;              /* 入口参数 */
    int64_t result;           /* 运行结果 */
    int active;               /* 是否已被注册 */
    int done;                 /* 是否已完成 */
} CoroTask;

#define MAX_CORO_TASKS 256
static CoroTask g_coro_tasks[MAX_CORO_TASKS];
static int64_t g_coro_count = 0;

/* 重置调度器（供测试/REPL 使用） */
void rt_coro_reset(void) {
    g_coro_count = 0;
    for (int64_t i = 0; i < MAX_CORO_TASKS; i++) {
        g_coro_tasks[i].fn = NULL;
        g_coro_tasks[i].result = 0;
        g_coro_tasks[i].active = 0;
        g_coro_tasks[i].done = 0;
    }
}

/* 获取当前已注册协程数量 */
int64_t rt_coro_count(void) {
    return g_coro_count;
}

/* 注册一个新协程任务，返回 handle（>=0 成功，-1 失败） */
int64_t rt_coro_spawn(void* fn, int64_t arg) {
    if (g_coro_count >= MAX_CORO_TASKS) return -1;
    for (int64_t i = 0; i < MAX_CORO_TASKS; i++) {
        if (g_coro_tasks[i].active == 0) {
            g_coro_tasks[i].fn = (int64_t(*)(int64_t))fn;
            g_coro_tasks[i].arg = arg;
            g_coro_tasks[i].result = 0;
            g_coro_tasks[i].active = 1;
            g_coro_tasks[i].done = 0;
            g_coro_count++;
            return i;
        }
    }
    return -1;
}

/* 是否存在指定协程任务 */
int64_t rt_coro_exists(int64_t handle) {
    if (handle < 0 || handle >= MAX_CORO_TASKS) return 0;
    return g_coro_tasks[handle].active;
}

/* 协程是否已完成 */
int64_t rt_coro_is_done(int64_t handle) {
    if (handle < 0 || handle >= MAX_CORO_TASKS) return 1;
    return g_coro_tasks[handle].done;
}

/* 运行一个协程任务直至完成（同步运行，供等待 使用） */
int64_t rt_coro_resume(int64_t handle) {
    if (handle < 0 || handle >= MAX_CORO_TASKS || !g_coro_tasks[handle].active) return 0;
    CoroTask* task = &g_coro_tasks[handle];
    if (task->done) return task->result;
    /* 简单协程：函数执行到返回视为一次 resume 完成 */
    task->result = task->fn ? task->fn(task->arg) : 0;
    task->done = 1;
    return task->result;
}

/* 等待某个协程完成（真正的 await 语义） */
int64_t rt_coro_await(int64_t handle) {
    return rt_coro_resume(handle);
}

/* 轮流推进所有未完成的协程（一轮 yield），返回本轮推进的数量 */
int64_t rt_coro_tick(void) {
    int64_t progressed = 0;
    for (int64_t i = 0; i < MAX_CORO_TASKS; i++) {
        if (g_coro_tasks[i].active && !g_coro_tasks[i].done) {
            rt_coro_resume(i);
            progressed++;
        }
    }
    return progressed;
}

/* 运行全部已注册协程直至完成 */
void rt_coro_run_all(void) {
    for (int64_t i = 0; i < MAX_CORO_TASKS; i++) {
        if (g_coro_tasks[i].active && !g_coro_tasks[i].done) {
            rt_coro_resume(i);
        }
    }
}

/* ================================================================
 * 调试器陷阱（v0.3.0 最小版运行时调试支持）
 * ----------------------------------------------------------------
 * 编译时在 --debug 模式下，codegen 在每个语句前注入
 * call @rt_debug_trap(line, func_name)，运行时通过 stdin
 * 接收调试命令（继续/单步/退出）。
 * ================================================================ */

#include <stdint.h>

/* 调试模式是否启用（由 codegen 在 main 前调用 rt_debug_init 设置） */
static int g_debug_enabled = 0;
/* 是否单步模式（每行都停下） */
static int g_debug_step_mode = 0;
/* 断点行号集合（最多 64 个断点） */
static int64_t g_breakpoints[64];
static int g_breakpoint_count = 0;

/* 启用调试模式 */
void rt_debug_init(void) {
    g_debug_enabled = 1;
    g_debug_step_mode = 1;  /* 默认进入单步模式，在第一条语句处停下 */
}

/* 添加断点 */
void rt_debug_add_breakpoint(int64_t line) {
    if (g_breakpoint_count < 64) {
        g_breakpoints[g_breakpoint_count++] = line;
    }
}

/* 检查是否在断点处 */
static int is_breakpoint(int64_t line) {
    for (int i = 0; i < g_breakpoint_count; i++) {
        if (g_breakpoints[i] == line) return 1;
    }
    return 0;
}

/* 调试陷阱：在每条语句执行前调用 */
void rt_debug_trap(int64_t line, const char* func_name) {
    if (!g_debug_enabled) return;

    /* 检查是否需要在此行停下（首次调用时自动进入单步模式提示） */
    if (!g_debug_step_mode && !is_breakpoint(line)) return;

    fprintf(stderr, "\n[调试] 行 %lld, 函数 '%s'\n", (long long)line, func_name ? func_name : "??");
    fflush(stderr);
    g_debug_step_mode = 0;  /* 单步后清除步进标志 */

    /* 读取调试命令 */
    char cmd[256];
    for (;;) {
        fprintf(stderr, "(xy-dbg) ");
        fflush(stderr);
        if (!fgets(cmd, sizeof(cmd), stdin)) break;

        /* 去除换行 */
        size_t len = strlen(cmd);
        if (len > 0 && cmd[len-1] == '\n') cmd[len-1] = '\0';

        if (strcmp(cmd, "c") == 0 || strcmp(cmd, "继续") == 0 || strcmp(cmd, "continue") == 0) {
            return;
        } else if (strcmp(cmd, "s") == 0 || strcmp(cmd, "单步") == 0 || strcmp(cmd, "step") == 0) {
            g_debug_step_mode = 1;
            return;
        } else if (strcmp(cmd, "q") == 0 || strcmp(cmd, "退出") == 0 || strcmp(cmd, "quit") == 0) {
            fprintf(stderr, "[调试] 退出程序\n");
            fflush(stderr);
            exit(0);
        } else if (strcmp(cmd, "h") == 0 || strcmp(cmd, "帮助") == 0 || strcmp(cmd, "help") == 0) {
            fprintf(stderr, "  命令:\n");
            fprintf(stderr, "    c / 继续 / continue  - 继续执行\n");
            fprintf(stderr, "    s / 单步 / step      - 单步执行\n");
            fprintf(stderr, "    q / 退出 / quit      - 退出程序\n");
            fprintf(stderr, "    h / 帮助 / help      - 显示帮助\n");
        } else if (strlen(cmd) > 0) {
            fprintf(stderr, "  未知命令: '%s' (输入 'h' 查看帮助)\n", cmd);
        }
        fflush(stderr);
    }
}

/* Entry point - provided by compiled IR module */