import 'package:flutter/material.dart';
import 'package:forui/forui.dart';

class VtopWebviewSearchAction extends StatelessWidget {
  const VtopWebviewSearchAction({required this.onPress, super.key});

  final VoidCallback onPress;

  @override
  Widget build(BuildContext context) {
    return FHeaderAction(
      icon: const Icon(FLucideIcons.search),
      semanticsLabel: 'Search VTOP pages',
      onPress: onPress,
    );
  }
}

class VtopWebviewActionsMenu extends StatelessWidget {
  const VtopWebviewActionsMenu({
    required this.isDarkMode,
    required this.isCompactMode,
    required this.isDesktopMode,
    required this.onSearch,
    required this.onHome,
    required this.onToggleDarkMode,
    required this.onToggleCompactMode,
    required this.onToggleDesktopMode,
    required this.onForceLogin,
    super.key,
  });

  final bool isDarkMode;
  final bool isCompactMode;
  final bool isDesktopMode;
  final VoidCallback onSearch;
  final VoidCallback onHome;
  final VoidCallback onToggleDarkMode;
  final VoidCallback onToggleCompactMode;
  final VoidCallback onToggleDesktopMode;
  final VoidCallback onForceLogin;

  @override
  Widget build(BuildContext context) {
    return FPopoverMenu(
      autofocus: true,
      menuAnchor: Alignment.topRight,
      childAnchor: Alignment.bottomRight,
      menu: [
        FItemGroup(
          children: [
            FItem(
              prefix: const Icon(FLucideIcons.search),
              title: const Text('Search pages'),
              onPress: onSearch,
            ),
            FItem(
              prefix: const Icon(FLucideIcons.house),
              title: const Text('VTOP Home'),
              onPress: onHome,
            ),
          ],
        ),
        FItemGroup(
          children: [
            FItem(
              prefix: FCheckbox(value: isDarkMode),
              title: const Text('Dark Pages'),
              onPress: onToggleDarkMode,
            ),
            FItem(
              prefix: FCheckbox(value: isCompactMode),
              title: const Text('Compact View'),
              onPress: onToggleCompactMode,
            ),
            FItem(
              prefix: FCheckbox(value: isDesktopMode),
              title: const Text('Desktop Mode'),
              onPress: onToggleDesktopMode,
            ),
          ],
        ),
        FItemGroup(
          children: [
            FItem(
              prefix: const Icon(FLucideIcons.logIn),
              title: const Text('Force Login'),
              onPress: onForceLogin,
            ),
          ],
        ),
      ],
      builder: (_, controller, _) => FHeaderAction(
        icon: const Icon(FLucideIcons.ellipsis),
        onPress: controller.toggle,
      ),
    );
  }
}
