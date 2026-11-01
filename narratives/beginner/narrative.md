i am a beginner i want to create a program that uses openssl
i ran first
un install un/openssl 
# just like php composer we come from github

then i write something simple main.cpp
#include <openssl.h>
int main(){
    OPENSSL_Init();
    return 0;
}

write the simple un.un just like in the example
# namespace sample; recomended to avoid conflicts
import openssl from un/openssl;
exec
.add(sample main.cpp)
.link(openssl::openssl);
# that implies that un should know 
# where to find openssl naturally without 
# the beginner having to tell anything to it;
# either some sort of global place to look for
# or locally installed at vendor/

then i simply do

un run sample

and wow worked