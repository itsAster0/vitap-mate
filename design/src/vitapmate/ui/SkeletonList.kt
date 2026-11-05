package vitapmate.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp

/** A card-shaped skeleton, repeated [count] times, for list screens. */
@Composable
fun SkeletonList(modifier: Modifier = Modifier, count: Int = 5, height: Dp = 96.dp) {
    Column(modifier) {
        repeat(count) {
            SectionCard(Modifier.fillMaxWidth().padding(bottom = Space.md)) {
                Row(Modifier.heightIn(min = height - Space.lg * 2), verticalAlignment = Alignment.CenterVertically) {
                    Skeleton(width = 44.dp, height = 44.dp, radius = Radii.md)
                    Spacer(Modifier.width(Space.md))
                    Column(Modifier.weight(1f), verticalArrangement = Arrangement.Center) {
                        Skeleton(width = 180.dp, height = 14.dp)
                        Spacer(Modifier.height(Space.sm))
                        Skeleton(width = 110.dp, height = 11.dp)
                    }
                }
            }
        }
    }
}
