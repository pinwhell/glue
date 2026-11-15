#ifndef FASTMATH_H
#define FASTMATH_H

#if defined(_WIN32) || defined(__CYGWIN__)
  #if defined(FASTMATH_EXPORTS)
    #define FASTMATH_API __declspec(dllexport)
  #else
    #define FASTMATH_API __declspec(dllimport)
  #endif
#else
  #if defined(__GNUC__) && __GNUC__ >= 4
    #define FASTMATH_API __attribute__((visibility("default")))
  #else
    #define FASTMATH_API
  #endif
#endif

#ifdef __cplusplus
extern "C" {
#endif

FASTMATH_API int fastmath_multiply(int a, int b);
FASTMATH_API int fastmath_power(int base, int exp);

#ifdef __cplusplus
}
#endif

#endif /* FASTMATH_H */
