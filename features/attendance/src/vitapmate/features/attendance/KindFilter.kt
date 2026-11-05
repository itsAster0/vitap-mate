package vitapmate.features.attendance

import com.composables.icons.lucide.FlaskConical
import com.composables.icons.lucide.BookOpen
import com.composables.icons.lucide.LayoutGrid
import com.composables.icons.lucide.Lucide
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import vitapmate.ui.ConnectedChoice

/** Which courses the list shows. */
internal enum class Kind { All, Theory, Lab }

/** All / Theory / Lab with their counts, as connected toggle buttons. */
@Composable
internal fun KindFilter(selected: Kind, counts: Map<Kind, Int>, onSelect: (Kind) -> Unit, modifier: Modifier = Modifier) = ConnectedChoice(
    Kind.entries,
    selected,
    onSelect,
    tag = "attendance.filter",
    key = { it.name.lowercase() },
    label = { "${it.name} ${counts[it] ?: 0}" },
    icon = { when (it) { Kind.All -> Lucide.LayoutGrid; Kind.Theory -> Lucide.BookOpen; Kind.Lab -> Lucide.FlaskConical } },
    labelOnSelected = true,
    modifier = modifier,
)
