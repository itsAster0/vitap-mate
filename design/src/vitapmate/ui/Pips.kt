package vitapmate.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

/** A segmented meter: [total] short bars, the first [filled] in the accent (e.g. 3 of 10 keys). */
@Composable
fun Pips(filled: Int, total: Int, modifier: Modifier = Modifier) {
    val empty = MaterialTheme.colorScheme.surfaceContainerHighest
    Row(modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(3.dp)) {
        repeat(total) { Box(Modifier.weight(1f).height(6.dp).background(if (it < filled) AppTheme.tones.accent else empty, CircleShape)) }
    }
}
