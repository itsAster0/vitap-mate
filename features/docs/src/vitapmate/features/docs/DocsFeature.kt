package vitapmate.features.docs

import com.composables.icons.lucide.Files
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Feature

/** Docs: saved files (VTOP downloads, outing passes, imports) and the mess menu; the viewer is its detail. */
val docsFeature = Feature("docs", "Docs", Lucide.Files, detail = { DocViewer(it) }) { DocsScreen() }
