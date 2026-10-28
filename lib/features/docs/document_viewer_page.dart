import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:forui/forui.dart';
import 'package:hooks_riverpod/hooks_riverpod.dart';
import 'package:pdfx/pdfx.dart' as pdfx;
import 'package:shared_preferences/shared_preferences.dart';
import 'package:vitapmate/ui/ui.dart';
import 'package:vitapmate/app/theme_provider.dart';
import 'package:vitapmate/features/docs/logic/doc_models.dart';
import 'package:vitapmate/features/docs/logic/spreadsheet_tables.dart';
import 'package:vitapmate/features/docs/logic/document_transform.dart';
import 'package:vitapmate/features/docs/docs_provider.dart';
import 'package:vitapmate/features/docs/widgets/mess_menu_view.dart';
import 'package:vitapmate/features/docs/widgets/pdf_document_viewport.dart';

part 'document_viewer/pdf.dart';
part 'document_viewer/content.dart';

class DocumentViewerPage extends HookConsumerWidget {
  final DocWindow doc;
  final bool embedded;

  const DocumentViewerPage({
    super.key,
    required this.doc,
    this.embedded = false,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    // "System" follows the phone's setting, not just an explicit Dark.
    final darkMode = switch (ref.watch(themeProvider)) {
      .dark => true,
      .system => MediaQuery.platformBrightnessOf(context) == .dark,
      .light => false,
    };
    final activeTitle = ref.read(activeDocumentTitleProvider.notifier);
    final registry = ref.read(docsRegistryProvider.notifier);

    useEffect(() {
      if (embedded) return null;
      Future.microtask(() {
        activeTitle.show(doc.name);
        unawaited(registry.touchLastOpened(doc.id));
      });
      return () {
        Future.microtask(() => activeTitle.clear(doc.name));
      };
    }, [doc.id, doc.name, embedded]);

    final transform = useMemoized(() {
      final m = Matrix4.diagonal3Values(doc.scale, doc.scale, 1)
        ..setTranslationRaw(doc.offsetX, doc.offsetY, 0);
      // A stale PDF offset can leave every page out of view; see recenterPdf.
      return TransformationController(doc.kind == .pdf ? recenterPdf(m) : m);
    }, [doc.id]);
    useEffect(() => transform.dispose, [transform]);

    if (!doc.hasFile) return _EmptyPrompt(doc: doc);
    return _DocSurface(
      doc: doc,
      darkMode: darkMode,
      transform: transform,
      embedded: embedded,
    );
  }
}

class _EmptyPrompt extends StatelessWidget {
  final DocWindow doc;
  const _EmptyPrompt({required this.doc});

  @override
  Widget build(BuildContext context) => Center(
    child: Column(
      mainAxisAlignment: .center,
      children: [
        Container(
          padding: const .all(24),
          decoration: BoxDecoration(
            color: context.theme.colors.primary.withValues(alpha: 0.1),
            shape: .circle,
          ),
          child: Icon(
            FLucideIcons.utensils,
            size: 40,
            color: context.theme.colors.primary,
          ),
        ),
        const SizedBox(height: 16),
        Text(
          doc.name,
          style: TextStyle(
            fontSize: 17,
            fontWeight: .w700,
            color: context.theme.colors.primary,
          ),
        ),
        const SizedBox(height: 8),
        Padding(
          padding: const .symmetric(horizontal: 40),
          child: Text(
            'This document has no file. Delete it and import the file again.',
            textAlign: .center,
            style: TextStyle(color: context.theme.colors.mutedForeground),
          ),
        ),
      ],
    ),
  );
}

class _DocSurface extends HookConsumerWidget {
  final DocWindow doc;
  final bool darkMode;
  final TransformationController transform;
  final bool embedded;

  const _DocSurface({
    required this.doc,
    required this.darkMode,
    required this.transform,
    required this.embedded,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final dpr = MediaQuery.devicePixelRatioOf(context);
    final searchOpen = useState(false);
    final searchQuery = useState('');
    final searchController = useTextEditingController();
    final searchFocus = useFocusNode();
    final showGestureHint = useState(false);
    final preSearchTransform = useRef<Matrix4?>(null);
    final saveTimer = useRef<Timer?>(null);
    final lastScrollOffset = useRef(doc.scrollOffset);
    final pendingSave =
        useRef<
          ({double scale, double offsetX, double offsetY, double scrollOffset})?
        >(null);
    final registry = ref.read(docsRegistryProvider.notifier);
    final messMenu = doc.kind == .spreadsheet && doc.asMessMenu;
    final searchable =
        (doc.kind == .spreadsheet && !messMenu) || doc.kind == .text;

    useEffect(() {
      if (embedded) return null;
      var cancelled = false;
      Timer? dismissTimer;
      SharedPreferences.getInstance().then((prefs) {
        if (cancelled ||
            (prefs.getBool('docs_viewer_gesture_hint_seen') ?? false)) {
          return;
        }
        showGestureHint.value = true;
        unawaited(prefs.setBool('docs_viewer_gesture_hint_seen', true));
        dismissTimer = Timer(const Duration(seconds: 4), () {
          if (!cancelled) showGestureHint.value = false;
        });
      });
      return () {
        cancelled = true;
        dismissTimer?.cancel();
      };
    }, [embedded]);

    void flushSave() {
      saveTimer.value?.cancel();
      saveTimer.value = null;
      final state = pendingSave.value;
      if (state == null) return;
      pendingSave.value = null;
      unawaited(
        registry.saveScrollState(
          doc.id,
          scale: state.scale,
          offsetX: state.offsetX,
          offsetY: state.offsetY,
          scrollOffset: state.scrollOffset,
        ),
      );
    }

    void persist({double scrollOffset = -1}) {
      showGestureHint.value = false;
      final m = transform.value;
      if (scrollOffset >= 0) lastScrollOffset.value = scrollOffset;
      pendingSave.value = (
        scale: m.getMaxScaleOnAxis(),
        offsetX: m.getTranslation().x,
        offsetY: m.getTranslation().y,
        scrollOffset: lastScrollOffset.value,
      );
      saveTimer.value?.cancel();
      saveTimer.value = Timer(const Duration(milliseconds: 500), flushSave);
    }

    useEffect(() => flushSave, []);

    final content = switch (doc.kind) {
      .pdf => _PdfContent(
        doc: doc,
        dpr: dpr,
        transform: transform,
        persist: persist,
      ),
      .spreadsheet when doc.asMessMenu => MessMenuView(
        doc: doc,
        compact: embedded,
      ),
      .spreadsheet => _SpreadsheetBody(
        doc: doc,
        darkMode: darkMode,
        transform: transform,
        persist: persist,
        searchQuery: searchQuery.value,
      ),
      .image || .text || .none => _SingleContent(
        doc: doc,
        darkMode: darkMode,
        dpr: dpr,
        transform: transform,
        persist: persist,
        searchQuery: searchQuery.value,
      ),
      .file => const Center(
        child: Text('Open this file from Docs with another app.'),
      ),
    };

    return Stack(
      children: [
        Positioned.fill(child: content),
        if (!embedded && searchable)
          Positioned(
            top: 12,
            right: 12,
            child: _ViewerSearch(
              open: searchOpen.value,
              controller: searchController,
              focusNode: searchFocus,
              onOpen: () {
                preSearchTransform.value = Matrix4.copy(transform.value);
                searchOpen.value = true;
                WidgetsBinding.instance.addPostFrameCallback(
                  (_) => searchFocus.requestFocus(),
                );
              },
              onChanged: (value) {
                searchQuery.value = value;
                if (value.trim().isNotEmpty) {
                  transform.value = Matrix4.identity();
                }
              },
              onClose: () {
                searchController.clear();
                searchQuery.value = '';
                searchOpen.value = false;
                searchFocus.unfocus();
                final previous = preSearchTransform.value;
                if (previous != null) transform.value = previous;
                preSearchTransform.value = null;
              },
            ),
          ),
        if (showGestureHint.value && !messMenu)
          Positioned(
            left: 24,
            right: 24,
            bottom: 22,
            child: IgnorePointer(
              child: Center(
                child: Container(
                  padding: const .symmetric(horizontal: 14, vertical: 10),
                  decoration: BoxDecoration(
                    color: context.theme.colors.primary,
                    borderRadius: .circular(999),
                    boxShadow: const [
                      BoxShadow(
                        color: Color(0x33000000),
                        blurRadius: 10,
                        offset: Offset(0, 3),
                      ),
                    ],
                  ),
                  child: Text(
                    'Drag to move  •  Pinch to zoom',
                    style: TextStyle(
                      color: context.theme.colors.primaryForeground,
                      fontSize: 12,
                      fontWeight: .w700,
                    ),
                  ),
                ),
              ),
            ),
          ),
        // The mess menu is a plain list; there is nothing to pan or re-centre.
        if (!messMenu)
          Positioned(
            right: 12,
            bottom: 12,
            child: FTappable(
              onPress: () {
                transform.value = doc.kind == .pdf
                    ? recenterPdf(transform.value)
                    : resetHorizontalOffset(transform.value);
                persist();
              },
              child: Container(
                padding: const .all(10),
                decoration: _controlDecoration(
                  context,
                  darkMode
                      ? context.theme.colors.primaryForeground
                      : Colors.white,
                ),
                child: Icon(
                  FLucideIcons.locateFixed,
                  size: 18,
                  color: context.theme.colors.primary,
                ),
              ),
            ),
          ),
      ],
    );
  }
}

class _ViewerSearch extends StatelessWidget {
  final bool open;
  final TextEditingController controller;
  final FocusNode focusNode;
  final VoidCallback onOpen;
  final ValueChanged<String> onChanged;
  final VoidCallback onClose;

  const _ViewerSearch({
    required this.open,
    required this.controller,
    required this.focusNode,
    required this.onOpen,
    required this.onChanged,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    if (!open) {
      return FTappable(
        onPress: onOpen,
        child: _ViewerControlSurface(
          child: Icon(
            Icons.search_rounded,
            size: 20,
            color: context.theme.colors.primary,
          ),
        ),
      );
    }

    return Container(
      width: (MediaQuery.sizeOf(context).width - 40).clamp(180.0, 340.0),
      height: 44,
      decoration: _controlDecoration(context, context.theme.colors.background),
      child: Row(
        children: [
          const SizedBox(width: 11),
          Icon(
            Icons.search_rounded,
            size: 19,
            color: context.theme.colors.mutedForeground,
          ),
          const SizedBox(width: 8),
          Expanded(
            child: TextField(
              focusNode: focusNode,
              controller: controller,
              onChanged: onChanged,
              style: TextStyle(
                fontSize: 13,
                color: context.theme.colors.primary,
              ),
              decoration: const InputDecoration(
                hintText: 'Search document',
                border: InputBorder.none,
                isDense: true,
                contentPadding: .zero,
              ),
            ),
          ),
          IconButton(
            onPressed: onClose,
            tooltip: 'Close search',
            icon: const Icon(Icons.close_rounded, size: 18),
          ),
        ],
      ),
    );
  }
}

/// The rounded, bordered, lightly shadowed box under the viewer's controls.
BoxDecoration _controlDecoration(BuildContext context, Color color) =>
    BoxDecoration(
      color: color,
      borderRadius: .circular(12),
      border: Border.all(color: context.theme.colors.border),
      boxShadow: const [
        BoxShadow(
          color: Color(0x22000000),
          blurRadius: 8,
          offset: Offset(0, 2),
        ),
      ],
    );

class _ViewerControlSurface extends StatelessWidget {
  final Widget child;

  const _ViewerControlSurface({required this.child});

  @override
  Widget build(BuildContext context) => Container(
    width: 44,
    height: 44,
    alignment: .center,
    decoration: _controlDecoration(context, context.theme.colors.background),
    child: child,
  );
}

/// Where [doc]'s file is stored, or null when it is missing.
AsyncSnapshot<String?> _useStoredPath(WidgetRef ref, DocWindow doc) =>
    useFuture(
      useMemoized(() async {
        final repo = await ref.read(docsRepositoryProvider.future);
        return repo.storedFilePathOf(doc);
      }, [doc.fileName]),
    );

/// A short grey message in the middle of the viewer.
class _Notice extends StatelessWidget {
  final String text;

  const _Notice(this.text);

  @override
  Widget build(BuildContext context) => Center(
    child: Text(
      text,
      style: TextStyle(color: context.theme.colors.mutedForeground),
    ),
  );
}
