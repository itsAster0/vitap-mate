package vitapmate.features.docs

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import com.composables.icons.lucide.Files
import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Utensils
import vitapmate.data.Docs
import vitapmate.data.newestMessMenu
import vitapmate.ui.EmptyView
import vitapmate.ui.Feature

/** Docs: saved files (VTOP downloads, outing passes, imports) and the mess menu; the viewer is its detail. */
val docsFeature = Feature("docs", "Docs", Lucide.Files, detail = { DocViewer(it) }) { DocsScreen() }

/** The newest mess menu full screen. */
val messFeature = Feature("mess", "Mess", Lucide.Utensils) { MessMenu() }

/** The newest mess menu: the whole month, or with [mealOnly] the meal on now (or next) only; where to add one while there is none. */
@Composable
fun MessMenu(mealOnly: Boolean = false) {
    if (!Docs.isLoaded) return Loading()
    val doc = Docs.docs.newestMessMenu()
        ?: return EmptyView(Lucide.Utensils, "No mess menu yet", Modifier.testTag("mess.empty"), message = "Import the menu spreadsheet in More → Docs as a mess menu.")
    // The menu leaves room for the nav bar itself and scrolls under it.
    Box(Modifier.fillMaxSize().testTag("mess.menu")) { DocContent(doc, embedded = mealOnly, inTab = true) }
}
