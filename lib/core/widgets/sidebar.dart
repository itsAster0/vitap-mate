import 'package:flutter/widgets.dart';
import 'package:forui/forui.dart';
import 'package:go_router/go_router.dart';
import 'package:vitapmate/core/router/paths.dart';

class Sidebar extends StatelessWidget {
  final FLayout side;

  const Sidebar({required this.side, super.key});

  @override
  Widget build(BuildContext context) => Container(
    height: double.infinity,
    width: double.infinity,
    decoration: BoxDecoration(
      color: context.theme.colors.background,
      border:
          side.vertical
              ? Border.symmetric(
                horizontal: BorderSide(color: context.theme.colors.border),
              )
              : Border.symmetric(
                vertical: BorderSide(color: context.theme.colors.border),
              ),
    ),
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 15, vertical: 8.0),
      child: Center(
        child: Column(
          mainAxisSize: MainAxisSize.max,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SizedBox(height: MediaQuery.of(context).size.height * 0.1),
            FTileGroup(
              label: const Text(''),
              description: const Text('Settings'),
              children: [
                FTile(
                  prefixIcon: Icon(FIcons.user),
                  title: const Text('Vtop Details'),
                  suffixIcon: Icon(FIcons.chevronRight),
                  onPress: () {
                    GoRouter.of(context).pushNamed(Paths.vtopUserManagement);
                  },
                ),
                // FTile(
                //   prefixIcon: Icon(FIcons.feather),
                //   title: const Text(''),
                //   suffixIcon: Icon(FIcons.chevronRight),
                //   onPress: () {},
                // ),
              ],
            ),
          ],
        ),
      ),
    ),
  );
}
