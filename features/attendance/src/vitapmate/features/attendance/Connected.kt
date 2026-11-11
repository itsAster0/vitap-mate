package vitapmate.features.attendance

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.unit.Dp

/**
 * The shape of item [i] of [count] in a connected group: [outer] corners at the group's ends,
 * [inner] where it meets its neighbours. A row unless [vertical].
 */
internal fun connected(i: Int, count: Int, outer: Dp, inner: Dp, vertical: Boolean = false): RoundedCornerShape {
    val start = if (i == 0) outer else inner
    val end = if (i == count - 1) outer else inner
    return if (vertical) RoundedCornerShape(start, start, end, end) else RoundedCornerShape(start, end, end, start)
}
