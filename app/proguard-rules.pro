# Release is minified (the toolchain turns on R8 and resource shrinking).
# JNA finds natives, Structure fields and callbacks by name through reflection.
-dontwarn java.awt.**
-keep class com.sun.jna.** { *; }
-keep class * extends com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
# UniFFI bindings: UniffiLib's external funs are registered with JNA by name.
#-keep class vitapmate.core.** { *; }
