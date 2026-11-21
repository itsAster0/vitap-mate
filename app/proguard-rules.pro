# Release is minified (the toolchain turns on R8 and resource shrinking).
# JNA finds natives, Structure fields and callbacks by name through reflection (Windows-only win32 stays out).
-dontwarn java.awt.**
-keep class !com.sun.jna.win32.**,com.sun.jna.** { *; }
-keep class * extends com.sun.jna.** { *; }
-keepclassmembers class * extends com.sun.jna.** { public *; }
# Strip Kotlin compiler null-check parameter/expression assertions and their name strings in release.
-assumenosideeffects class kotlin.jvm.internal.Intrinsics {
    public static void checkNotNull(java.lang.Object);
    public static void checkNotNull(java.lang.Object, java.lang.String);
    public static void checkExpressionValueIsNotNull(java.lang.Object, java.lang.String);
    public static void checkNotNullExpressionValue(java.lang.Object, java.lang.String);
    public static void checkParameterIsNotNull(java.lang.Object, java.lang.String);
    public static void checkNotNullParameter(java.lang.Object, java.lang.String);
    public static void checkReturnedValueIsNotNull(java.lang.Object, java.lang.String);
    public static void checkFieldIsNotNull(java.lang.Object, java.lang.String);
}
# UniFFI bindings: UniffiLib's external funs are registered with JNA by name.
#-keep class vitapmate.core.** { *; }
