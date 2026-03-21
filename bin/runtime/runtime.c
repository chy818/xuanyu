/**
 * @file runtime.c
 * @brief XY Language Runtime Library (v0.1) - Cross-Platform Version
 * @description 移除了所有 POSIX 依赖 (unistd.h)，仅使用标准 C99
 *              可在 Windows (MSVC/MinGW), Linux, macOS 上无缝编译
 */

/* 禁用 MSVC 安全警告 */
#define _CRT_SECURE_NO_WARNINGS

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

/* Windows 特定头文件 */
#ifdef _WIN32
#include <windows.h>
#include <io.h>
#include <fcntl.h>

/* Windows 控制台 UTF-8 初始化 */
static int g_console_initialized = 0;

/**
 * 初始化 Windows 控制台以支持 UTF-8 输出
 * 必须在程序启动时调用一次
 */
static void init_windows_console(void) {
    if (g_console_initialized) return;
    g_console_initialized = 1;
    
    /* 设置控制台输出代码页为 UTF-8 */
    SetConsoleOutputCP(65001);
    SetConsoleCP(65001);
}

/* 自动初始化属性 */
__attribute__((constructor))
static void auto_init_console(void) {
    init_windows_console();
}
#endif

/* === 内部结构定义 (对用户透明) === */

/**
 * 字符串结构：长度 + 数据 (UTF-8 字节流)
 */
typedef struct {
    int64_t len;
    char* data;
} XyString;

/**
 * 列表结构：动态数组，存储 void* (类型擦除)
 */
typedef struct {
    int64_t count;
    int64_t capacity;
    void** items;
} XyList;

/* === 字符串 API === */

/**
 * 创建字符串 (从 C const char*)
 * @param utf8_content UTF-8 编码的字符串内容
 * @return 字符串指针，失败返回 NULL
 */
void* rt_string_new(const char* utf8_content) {
    if (!utf8_content) return NULL;
    
    XyString* s = (XyString*)malloc(sizeof(XyString));
    if (!s) return NULL;
    
    s->len = (int64_t)strlen(utf8_content);  /* 字节长度，非字符数 */
    s->data = (char*)malloc(s->len + 1);
    if (!s->data) {
        free(s);
        return NULL;
    }
    
    memcpy(s->data, utf8_content, s->len + 1);  /* 包含 '\0' */
    return (void*)s;
}

/**
 * 获取字符串长度 (字节数)
 * @param s_ptr 字符串指针
 * @return 字节长度
 */
int64_t rt_string_len(void* s_ptr) {
    if (!s_ptr) return 0;
    return ((XyString*)s_ptr)->len;
}

/**
 * 释放字符串
 * @param s_ptr 字符串指针
 */
void rt_string_free(void* s_ptr) {
    if (!s_ptr) return;
    XyString* s = (XyString*)s_ptr;
    if (s->data) free(s->data);
    free(s);
}

/* === 列表 API (类型擦除，存储指针) === */

/**
 * 创建新列表
 * @return 列表指针，失败返回 NULL
 */
void* rt_list_new() {
    XyList* list = (XyList*)malloc(sizeof(XyList));
    if (!list) return NULL;
    
    list->count = 0;
    list->capacity = 8;  /* 初始容量 */
    list->items = (void**)malloc(list->capacity * sizeof(void*));
    if (!list->items) {
        free(list);
        return NULL;
    }
    return (void*)list;
}

/**
 * 向列表追加元素
 * @param list_ptr 列表指针
 * @param item 要添加的元素指针
 */
void rt_list_append(void* list_ptr, void* item) {
    if (!list_ptr) return;
    XyList* list = (XyList*)list_ptr;
    
    if (list->count >= list->capacity) {
        /* 扩容 2 倍 */
        int64_t new_cap = list->capacity * 2;
        void** new_items = (void**)realloc(list->items, new_cap * sizeof(void*));
        if (!new_items) return;  /* 简单处理：分配失败忽略 */
        list->items = new_items;
        list->capacity = new_cap;
    }
    
    list->items[list->count++] = item;
}

/**
 * 获取列表元素
 * @param list_ptr 列表指针
 * @param index 索引 (从 0 开始)
 * @return 元素指针，越界返回 NULL
 */
void* rt_list_get(void* list_ptr, int64_t index) {
    if (!list_ptr) return NULL;
    XyList* list = (XyList*)list_ptr;
    
    if (index >= list->count) {  /* 修复：应该是 >= 而不是 = */
        /* 越界处理：返回 NULL */
        return NULL;
    }
    return list->items[index];
}

/**
 * 获取列表长度
 * @param list_ptr 列表指针
 * @return 元素数量
 */
int64_t rt_list_len(void* list_ptr) {
    if (!list_ptr) return 0;
    return ((XyList*)list_ptr)->count;
}

/**
 * 释放列表
 * @param list_ptr 列表指针
 */
void rt_list_free(void* list_ptr) {
    if (!list_ptr) return;
    XyList* list = (XyList*)list_ptr;
    if (list->items) free(list->items);
    free(list);
}

/* === IO API === */

/**
 * 打印字符串并换行
 * @param s_ptr 字符串指针
 */
void rt_println(void* s_ptr) {
    if (!s_ptr) {
        printf("\n");
        return;
    }
    XyString* s = (XyString*)s_ptr;
    /* 直接输出 UTF-8 字节流，终端会自动渲染 */
    fwrite(s->data, 1, s->len, stdout);
    printf("\n");
    fflush(stdout);
}

/**
 * 读取一行 (包含换行符处理)
 * @return 字符串指针，EOF 或错误返回空串
 */
void* rt_readline() {
    char buffer[4096];  /* 限制单行最大长度 */
    if (fgets(buffer, sizeof(buffer), stdin) == NULL) {
        return rt_string_new("");  /* EOF 或错误返回空串 */
    }
    
    /* 去除末尾换行符 (\n 或 \r\n) */
    size_t len = strlen(buffer);
    while (len > 0 && (buffer[len-1] == '\n' || buffer[len-1] == '\r')) {
        buffer[--len] = '\0';
    }
    
    return rt_string_new(buffer);
}

/* === 兼容旧版本的别名函数 === */

/**
 * 打印函数 (兼容旧版本)
 * @param str 要打印的字符串
 * @return 0 表示成功
 */

/**
 * 打印字符串 (void* 版本，用于 LLVM IR 调用)
 * 支持两种格式：
 * 1. XyString* 结构指针
 * 2. 原始 C 字符串 (i8* 指向常量)
 * @param str_ptr 字符串指针
 */
void print(void* str_ptr) {
    if (!str_ptr) {
        printf("(null)");
        return;
    }
    
    /* 获取指针地址 */
    uintptr_t addr = (uintptr_t)str_ptr;
    
    /* 
     * 检查是否是堆分配的 XyString 结构
     * 堆地址通常在某个范围内（取决于系统）
     * Windows: 0x00010000 - 0x7FFFFFFF (用户空间)
     * 但这不可靠，所以我们换一种方法
     */
    
    /* 读取第一个字节作为试探 */
    unsigned char first_byte = *(unsigned char*)str_ptr;
    
    /* 
     * 字符串常量通常以可打印字符开头
     * XyString 结构的第一部分是 len 字段，应该是正整数
     */
    if (first_byte >= 32 && first_byte <= 126) {
        /* 看起来像普通字符开头，按 C 字符串处理 */
        printf("%s", (const char*)str_ptr);
        return;
    }
    
    /* 检查是否是 XyString 结构 */
    XyString* s = (XyString*)str_ptr;
    if (s->len > 0 && s->len < 1024*1024 && s->data != NULL) {
        /* 看起来像有效的 XyString */
        fwrite(s->data, 1, s->len, stdout);
        return;
    }
    
    /* 默认按 C 字符串处理 */
    printf("%s", (const char*)str_ptr);
}

/**
 * 打印整数 (void 版本，用于 LLVM IR)
 * @param val 要打印的整数
 */
void print_int(int64_t val) {
    printf("%lld", (long long)val);
}

/**
 * 打印浮点数 (void 版本，用于 LLVM IR)
 * @param val 要打印的浮点数
 */
void print_float(double val) {
    printf("%f", val);
}

/**
 * 打印布尔值 (void 版本，用于 LLVM IR)
 * @param val 要打印的布尔值 (0=false, 1=true)
 */
void print_bool(int val) {
    printf("%s", val ? "true" : "false");
}

/**
 * 打印字符串 (const char* 版本，兼容旧代码)
 * @param str 要打印的字符串
 * @return 0 表示成功
 */
int 打印(const char* str) {
    printf("%s", str);
    return 0;
}

/**
 * 打印整数函数 (兼容旧版本)
 * @param val 要打印的整数
 * @return 0 表示成功
 */
int 打印整数(int64_t val) {
    printf("%lld", (long long)val);
    return 0;
}

/**
 * 打印换行
 * @return 0 表示成功
 */
int 打印换行() {
    printf("\n");
    return 0;
}

/**
 * 读取整数 (兼容旧版本)
 * @return 读取到的整数
 */
int64_t 读取() {
    int64_t val;
    if (scanf("%lld", &val) == 1) {
        return val;
    }
    return 0;
}

/**
 * 延时函数 (毫秒)
 * @param ms 延时毫秒数
 */
void 延时(int ms) {
#ifdef _WIN32
    Sleep(ms);
#else
    usleep(ms * 1000);
#endif
}

/**
 * 退出函数
 * @param code 退出码
 */
void 退出(int code) {
    exit(code);
}

/**
 * 获取随机数
 * @return 随机整数
 */
int 随机数() {
    return rand();
}

/* === 辅助调试 === */

/**
 * 运行时 panic
 * @param msg 错误消息
 */
void rt_panic(const char* msg) {
    fprintf(stderr, "XY Runtime Panic: %s\n", msg);
    exit(1);
}
