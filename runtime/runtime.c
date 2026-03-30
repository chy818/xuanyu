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

/* Print functions */
void print(void* str) {
    if (str) printf("%s", (char*)str);
}

void print_int(int64_t val) {
    printf("%lld", val);
}

void print_float(double val) {
    printf("%g", val);
}

void print_bool(int val) {
    printf("%s", val ? "true" : "false");
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
int64_t input_int() {
    int64_t val;
    if (scanf("%lld", &val) == 1) return val;
    return 0;
}

void* input_text() {
    static char buffer[4096];
    if (fgets(buffer, sizeof(buffer), stdin)) {
        size_t len = strlen(buffer);
        if (len > 0 && buffer[len-1] == '\n') buffer[len-1] = '\0';
        return strdup(buffer);
    }
    return NULL;
}

/* Entry point - provided by compiled IR module */
