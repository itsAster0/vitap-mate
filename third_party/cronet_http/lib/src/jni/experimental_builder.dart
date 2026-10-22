// VITAP Mate patch: the two members of
// `org.chromium.net.ExperimentalCronetEngine$Builder` that
// `CronetEngine.build(experimentalOptions:)` needs, written in the style of
// the generated bindings (jni_bindings.dart has no ExperimentalCronetEngine).

// ignore_for_file: invalid_use_of_internal_member
// ignore_for_file: library_prefixes
// ignore_for_file: no_leading_underscores_for_library_prefixes

import 'package:jni/_internal.dart' as jni$_;
import 'package:jni/jni.dart' as jni$_;

import 'jni_bindings.dart' as jb;

final _class =
    jni$_.JClass.forName(r'org/chromium/net/ExperimentalCronetEngine$Builder');

final _idNew = _class.constructorId(r'(Landroid/content/Context;)V');

final _new = jni$_.ProtectedJniExtensions.lookup<
            jni$_.NativeFunction<
                jni$_.JniResult Function(
                    jni$_.Pointer<jni$_.Void>,
                    jni$_.JMethodIDPtr,
                    jni$_.VarArgs<(jni$_.Pointer<jni$_.Void>,)>)>>(
        'globalEnv_NewObject')
    .asFunction<
        jni$_.JniResult Function(jni$_.Pointer<jni$_.Void>, jni$_.JMethodIDPtr,
            jni$_.Pointer<jni$_.Void>)>();

final _idSetExperimentalOptions = _class.instanceMethodId(
  r'setExperimentalOptions',
  r'(Ljava/lang/String;)Lorg/chromium/net/ExperimentalCronetEngine$Builder;',
);

final _setExperimentalOptions = jni$_.ProtectedJniExtensions.lookup<
            jni$_.NativeFunction<
                jni$_.JniResult Function(
                    jni$_.Pointer<jni$_.Void>,
                    jni$_.JMethodIDPtr,
                    jni$_.VarArgs<(jni$_.Pointer<jni$_.Void>,)>)>>(
        'globalEnv_CallObjectMethod')
    .asFunction<
        jni$_.JniResult Function(jni$_.Pointer<jni$_.Void>, jni$_.JMethodIDPtr,
            jni$_.Pointer<jni$_.Void>)>();

/// `new ExperimentalCronetEngine.Builder(context)`, as the plain builder type
/// (it is a subclass), with [options] set: Cronet's experimental options
/// JSON. The returned object must be released after use.
jb.CronetEngine$Builder experimentalCronetEngineBuilder(
  jb.Context context,
  jni$_.JString options,
) {
  final builder = _new(_class.reference.pointer, _idNew.pointer,
          context.reference.pointer)
      .object<jb.CronetEngine$Builder>();
  _setExperimentalOptions(builder.reference.pointer,
          _idSetExperimentalOptions.pointer, options.reference.pointer)
      .object<jb.CronetEngine$Builder?>()
      ?.release();
  return builder;
}
