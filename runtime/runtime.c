/**
 * @file runtime.c
 * @brief CCAS 运行时库
 * @description 提供打印、读取、延时等基础库函数
 */

#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#ifdef _WIN32
#include <windows.h>
#endif

/**
 * 打印函数
 * 对应 LLVM IR: declare i32 @打印(i8*)
 * @param str 要打印的字符串
 * @return 0 表示成功
 */
int 打印(const char* str) {
    printf("%s", str);
    return 0;
}

/**
 * 打印整数函数
 * 对应 LLVM IR: declare i32 @打印整数(i32)
 * @param val 要打印的整数
 * @return 0 表示成功
 */
int 打印整数(int val) {
    printf("%d", val);
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
 * 读取整数函数
 * 对应 LLVM IR: declare i32 @读取()
 * @return 读取到的整数
 */
int 读取() {
    int val;
    if (scanf("%d", &val) == 1) {
        return val;
    }
    return 0;
}

/**
 * 延时函数 (毫秒)
 * 对应 LLVM IR: declare void @延时(i32)
 * @param ms 延时毫秒数
 */
#ifdef _WIN32
void 延时(int ms) {
    Sleep(ms);
}
#else
void 延时(int ms) {
    usleep(ms * 1000);
}
#endif

/**
 * 退出函数
 * 对应 LLVM IR: declare void @退出(i32)
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

/**
 * 获取当前时间 (毫秒)
 * @return 当前时间戳
 */
long long 当前时间() {
#ifdef _WIN32
    return GetTickCount();
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1000 + ts.tv_nsec / 1000000;
#endif
}
