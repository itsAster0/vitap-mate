import 'package:growthbook_sdk_flutter/growthbook_sdk_flutter.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';
import 'package:vitapmate/core/di/provider/vtop_user_provider.dart';
import 'package:vitapmate/core/utils/entity/vtop_user_entity.dart';
import 'package:flutter/foundation.dart';
part 'feature_flags.g.dart';

@Riverpod(keepAlive: true)
class Gb extends _$Gb {
  @override
  Future<GrowthBookSDK> build() async {
    VtopUserEntity user = await ref.watch(vtopUserProvider.future);
    final info = await PackageInfo.fromPlatform();
    int versionStringToNumber(String version) {
      final parts = version.split('.');
      final major = int.parse(parts[0]);
      final minor = parts.length > 1 ? int.parse(parts[1]) : 0;
      final patch = parts.length > 2 ? int.parse(parts[2]) : 0;
      final out = major * 1000000 + minor * 1000 + patch;
      return out;
    }

    int appVersionCode = versionStringToNumber(info.version);
    var userAttr = {"id": user.username, "appVersionCode": appVersionCode};
    GrowthBookSDK gb =
        await GBSDKBuilderApp(
          hostURL: 'https://api-gb.kryxen.dev/',
          apiKey: kDebugMode ? "sdk-8AKDpwpeGLRaImnj" : "sdk-mkfOz851tHlxJRM",
          attributes: userAttr,
          growthBookTrackingCallBack: (trackData) {},
          backgroundSync: true,
          remoteEval: false,
        ).initialize();
    return gb;
  }
}
