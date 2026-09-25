import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:forui/forui.dart';
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/core/di/provider/global_async_queue_provider.dart';
import 'package:vitapmate/core/providers/settings.dart';
import 'package:vitapmate/core/widgets/ui/ui.dart';

/// Screens that can reload their data. The floating refresh button runs the
/// newest one that is on screen, so a sheet beats the page under it and a
/// pushed page beats its parent.
class _ScreenRefreshRegistry extends ChangeNotifier {
  final _targets = <_ScreenRefreshState>[];

  _ScreenRefreshState? get active {
    for (final target in _targets.reversed) {
      if (target.isVisible) return target;
    }
    return null;
  }

  bool _pending = false;

  void _add(_ScreenRefreshState target) {
    _targets.add(target);
    _changed();
  }

  void _remove(_ScreenRefreshState target) {
    _targets.remove(target);
    _changed();
  }

  /// Screens register and unregister mid-build, so tell the button once the
  /// frame is done rather than rebuilding it during someone else's build.
  void _changed() {
    if (_pending) return;
    _pending = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _pending = false;
      notifyListeners();
    });
    WidgetsBinding.instance.ensureVisualUpdate();
  }
}

final _registry = _ScreenRefreshRegistry();

/// Marks [child] as a screen the floating refresh button can reload with
/// [onRefresh]. The callback handles its own errors.
class ScreenRefresh extends StatefulWidget {
  const ScreenRefresh({
    super.key,
    required this.onRefresh,
    this.tasks = const [],
    required this.child,
  });

  final Future<void> Function() onRefresh;

  /// Queue task id prefixes (e.g. 'vtop_marks') that load this screen's
  /// data. The button spins while any of them runs, however it was started.
  final List<String> tasks;
  final Widget child;

  @override
  State<ScreenRefresh> createState() => _ScreenRefreshState();
}

class _ScreenRefreshState extends State<ScreenRefresh> {
  ModalRoute<Object?>? _route;
  bool _tickerEnabled = true;

  /// Hidden tabs have tickers off; covered pages are not the current route.
  bool get isVisible =>
      mounted && _tickerEnabled && (_route?.isCurrent ?? true);

  Future<void> refresh() => widget.onRefresh();

  @override
  void initState() {
    super.initState();
    _registry._add(this);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final route = ModalRoute.of(context);
    if (route != _route) {
      _route?.secondaryAnimation?.removeStatusListener(_onCovered);
      // Fires when another page or sheet slides over this one, or leaves.
      route?.secondaryAnimation?.addStatusListener(_onCovered);
      _route = route;
    }
    final enabled = TickerMode.valuesOf(context).enabled;
    if (enabled != _tickerEnabled) {
      _tickerEnabled = enabled;
      _registry._changed();
    }
  }

  void _onCovered(AnimationStatus _) => _registry._changed();

  @override
  void dispose() {
    _route?.secondaryAnimation?.removeStatusListener(_onCovered);
    _registry._remove(this);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.child;
}

/// A droplet hanging off the right edge of every screen that reloads it.
/// Drag it up or down; where it is left is shared by all screens and kept
/// across restarts. Can be turned off in Settings.
class ScreenRefreshButton extends HookConsumerWidget {
  const ScreenRefreshButton({super.key});

  static const _width = 38.0;
  static const _height = 52.0;
  static const _prefsKey = 'refresh_button_y';

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final colors = context.theme.colors;
    final padding = MediaQuery.paddingOf(context);
    final size = MediaQuery.sizeOf(context);
    useListenable(_registry);
    final enabled = ref.watch(refreshButtonProvider);
    final target = enabled ? _registry.active : null;

    final pressed = useState(false);
    final dragging = useState(false);
    final running = ref.watch(
      globalAsyncQueueProvider.select((q) => q.running.keys.toList()),
    );
    final loading =
        pressed.value ||
        (target != null &&
            running.any((id) => target.widget.tasks.any(id.startsWith)));

    // Kept as a fraction of the travel so it survives rotation and different
    // screen sizes. 1 is the lowest spot, just above the bottom bar.
    final y = useState(0.85);
    useEffect(() {
      SharedPreferences.getInstance().then((prefs) {
        final saved = prefs.getDouble(_prefsKey);
        if (saved != null && context.mounted) y.value = saved;
      });
      return null;
    }, const []);

    final minTop = padding.top + Space.sm;
    final maxTop = size.height - padding.bottom - _height - 72;
    final travel = (maxTop - minTop).clamp(1.0, double.infinity);
    final top = minTop + travel * y.value;

    final spin = useAnimationController(
      duration: const Duration(milliseconds: 900),
    );
    useEffect(() {
      if (loading) {
        spin.repeat();
      } else if (spin.isAnimating) {
        // Finish the current turn instead of snapping back.
        spin.forward().then((_) => spin.reset());
      }
      return null;
    }, [loading]);

    Future<void> press() async {
      final current = _registry.active;
      if (current == null || pressed.value) return;
      HapticFeedback.lightImpact();
      pressed.value = true;
      try {
        await current.refresh();
      } catch (_) {
        // Screens report their own failures.
      } finally {
        if (context.mounted) pressed.value = false;
      }
    }

    final hidden = target == null;
    return Positioned(
      right: 0,
      top: top,
      child: IgnorePointer(
        ignoring: hidden,
        // Slides back into the edge when there is nothing to refresh.
        child: AnimatedSlide(
          duration: Motion.medium,
          curve: Curves.easeOutCubic,
          offset: hidden ? const Offset(1, 0) : Offset.zero,
          child: AnimatedScale(
            duration: Motion.fast,
            alignment: Alignment.centerRight,
            scale: dragging.value ? 1.08 : 1,
            child: GestureDetector(
              onVerticalDragStart: (_) {
                dragging.value = true;
                HapticFeedback.selectionClick();
              },
              onVerticalDragUpdate: (d) =>
                  y.value = (y.value + d.delta.dy / travel).clamp(0.0, 1.0),
              onVerticalDragEnd: (_) async {
                dragging.value = false;
                final prefs = await SharedPreferences.getInstance();
                await prefs.setDouble(_prefsKey, y.value);
              },
              child: Semantics(
                button: true,
                label: 'Refresh this screen',
                child: FTappable(
                  onPress: loading ? null : press,
                  child: CustomPaint(
                    size: const Size(_width, _height),
                    painter: _DropPainter(
                      fill: colors.secondary,
                      stroke: colors.border,
                    ),
                    child: SizedBox(
                      width: _width,
                      height: _height,
                      child: Align(
                        alignment: const Alignment(-0.12, 0),
                        child: RotationTransition(
                          turns: spin,
                          child: Icon(
                            FLucideIcons.refreshCw,
                            size: 16,
                            color: loading
                                ? colors.mutedForeground
                                : colors.foreground,
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// A drop clinging to the right wall: a round head on the left that flares
/// into the edge with soft concave fillets top and bottom.
class _DropPainter extends CustomPainter {
  const _DropPainter({required this.fill, required this.stroke});

  final Color fill;
  final Color stroke;

  Path _outline(Size size) {
    final w = size.width;
    final h = size.height;
    final r = h / 2 - 9;
    final cx = r + 2;
    final cy = h / 2;
    return Path()
      ..moveTo(w, 0)
      ..quadraticBezierTo(w, cy - r, cx + 2, cy - r)
      ..arcToPoint(
        Offset(cx + 2, cy + r),
        radius: Radius.circular(r),
        clockwise: false,
        largeArc: true,
      )
      ..quadraticBezierTo(w, cy + r, w, h);
  }

  @override
  void paint(Canvas canvas, Size size) {
    final open = _outline(size);
    final closed = Path.from(open)..close();
    canvas.drawShadow(closed, const Color(0xFF000000), 4, false);
    canvas.drawPath(closed, Paint()..color = fill);
    canvas.drawPath(
      open,
      Paint()
        ..color = stroke
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1,
    );
  }

  @override
  bool shouldRepaint(_DropPainter old) =>
      old.fill != fill || old.stroke != stroke;
}
