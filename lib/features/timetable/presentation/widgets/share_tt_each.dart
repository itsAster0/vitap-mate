import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:share_plus/share_plus.dart';
import 'package:vitapmate/core/utils/general_utils.dart';
import 'package:vitapmate/features/timetable/presentation/providers/pb_links.dart';

class ShareTtEach extends HookConsumerWidget {
  final List url;
  final String name;

  final String timetableId;
  final int updateTime;
  const ShareTtEach({
    super.key,
    required this.url,
    required this.name,
    required this.updateTime,
    required this.timetableId,
  });
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final selectedurl = useMemoized(() {
      for (var i in url) {
        if (i["timetable"].toString() == timetableId) {
          return i["secret_id"];
        }
      }
      return null;
    }, [url]);
    final colors = context.theme.colors;
    final isLinkAvilable = useMemoized(() {
      for (var i in url) {
        if (i["timetable"].toString() == timetableId) {
          return true;
        }
      }
      return false;
    }, [url]);
    final recordIdOfLink = useMemoized(() {
      for (var i in url) {
        if (i["timetable"].toString() == timetableId) {
          return i["id"].toString();
        }
      }
    }, [url]);
    final isLoading = useState(false);
    iconOrLoading(icon) {
      return isLoading.value ? FCircularProgress() : icon;
    }

    Future<T?> handler<T>(Future<T> Function() func) async {
      isLoading.value = true;

      try {
        final result = await func();
        return result;
      } catch (e) {
        if (context.mounted) {
          showDialog(
            context: context,
            builder:
                (BuildContext context) => FDialog(
                  title: Text("Error"),
                  body: Text(
                    "Your internet connection seems slow or unavailable.",
                  ),
                  actions: [
                    FButton(
                      onPress: () => Navigator.of(context).pop(),
                      child: const Text('OK'),
                    ),
                  ],
                ),
          );
        }
      } finally {
        if (context.mounted) {
          isLoading.value = false;
        }
      }
      return null;
    }

    void handelReload() async {
      if (isLoading.value) return;
      if (selectedurl != null) {
        bool? stop = await showFDialog<bool>(
          context: context,
          builder:
              (context, style, animation) => FDialog(
                title: const Text("Are you Sure?"),
                body: const Text("The previous link will not be valid anymore"),
                actions: [
                  FButton(
                    style: FButtonStyle.outline(),
                    onPress: () => Navigator.of(context).pop(true),
                    child: const Text('Cancel'),
                  ),
                  FButton(
                    onPress: () => Navigator.of(context).pop(false),
                    child: const Text('Continue'),
                  ),
                ],
              ),
        );
        if (stop == true) {
          return;
        }
      }

      await handler(() async {
        await ref.read(createTimetablelinkdbProvider(timetableId).future);
      });
    }

    void handeldel() {
      handler(() async {
        if (recordIdOfLink == null) return;
        await ref.read(delTimetablelinkdbProvider(recordIdOfLink).future);
      });
    }

    final urlShare = "https://va.kryxen.dev/timetable/$selectedurl";
    return Padding(
      padding: const EdgeInsets.all(8.0),
      child: Column(
        spacing: 8,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceAround,
            spacing: 5,
            children: [
              FButton.icon(
                onPress: handelReload,
                child: iconOrLoading(Icon(Icons.replay)),
              ),
              if (isLinkAvilable)
                FButton.icon(
                  onPress: () async {
                    await SharePlus.instance.share(
                      ShareParams(text: 'check out my timetable "$urlShare"'),
                    );
                  },
                  child: iconOrLoading(Icon(Icons.share)),
                ),
              if (isLinkAvilable)
                FButton.icon(
                  onPress: () async {
                    await Clipboard.setData(ClipboardData(text: urlShare));
                  },
                  child: iconOrLoading(Icon(Icons.copy)),
                ),
              if (isLinkAvilable)
                FButton.icon(
                  style: FButtonStyle.destructive(),
                  onPress: handeldel,
                  child: iconOrLoading(Icon(Icons.delete)),
                ),
            ],
          ),
          Text(
            "Last updated: ${formatUnixTimestamp(updateTime.toInt())}",
            textAlign: TextAlign.center,
            style: TextStyle(
              fontSize: 12,
              color: context.theme.colors.primary,
              fontWeight: FontWeight.w500,
            ),
          ),
        ],
      ),
    );
  }
}
