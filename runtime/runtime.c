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

/** List structure */
typedef struct {
    void** items;
    int64_t count;
    int64_t capacity;
} List;

/* Create new list */
void* rt_list_new() {
    List* list = (List*)malloc(sizeof(List));
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

/* Append to list */
void rt_list_append(void* list_ptr, void* item) {
    if (!list_ptr) return;
    List* list = (List*)list_ptr;
    if (list->count >= list->capacity) {
        int64_t new_cap = list->capacity == 0 ? 8 : list->capacity * 2;
        void** new_items = (void**)realloc(list->items, new_cap * sizeof(void*));
        if (!new_items) return;
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

/* Character to code conversion */
int64_t rt_char_to_code(void* ch_ptr) {
    if (!ch_ptr) return 0;
    char ch = *((char*)ch_ptr);
    return (int64_t)(unsigned char)ch;
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

void* rt_string_char_at(void* str, int64_t index) {
    if (!str) return NULL;
    char* s = (char*)str;
    int64_t len = strlen(s);
    if (index < 0 || index >= len) return strdup("");
    char* result = (char*)malloc(2);
    if (!result) return NULL;
    result[0] = s[index];
    result[1] = '\0';
    return result;
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
    if (start >= end) return strdup("");
    char* result = (char*)malloc(end - start + 1);
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
    FILE* f = fopen((char*)path, "r");
    if (!f) return NULL;
    fseek(f, 0, SEEK_END);
    long size = ftell(f);
    fseek(f, 0, SEEK_SET);
    char* result = (char*)malloc(size + 1);
    if (!result) { fclose(f); return NULL; }
    fread(result, 1, size, f);
    result[size] = '\0';
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

/* Entry point - provided by compiled IR module */