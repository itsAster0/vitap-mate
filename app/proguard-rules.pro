# Release is minified (the toolchain turns on R8 and resource shrinking).
# JNA finds natives, Structure fields and callbacks by name through reflection (Windows-only win32 stays out).
-dontwarn java.awt.**
-keep class !com.sun.jna.win32.**,com.sun.jna.** { *; }
-keep class * extends com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
# UniFFI bindings: UniffiLib's external funs are registered with JNA by name.
#-keep class vitapmate.core.** { *; }
