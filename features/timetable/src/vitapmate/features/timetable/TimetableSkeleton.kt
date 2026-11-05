package vitapmate.features.timetable

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import vitapmate.ui.Skeleton

/** The agenda's shape while the first copy loads: week strip, heading, focus card, three classes. */
@Composable
fun TimetableSkeleton() = Column(Modifier.fillMaxWidth().padding(horizontal = 8.dp, vertical = 8.dp).testTag("timetable.loading")) {
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween) {
        repeat(7) { Skeleton(width = 40.dp, height = 58.dp, radius = 20.dp) }
    }
    Spacer(Modifier.height(20.dp))
    Skeleton(width = 160.dp, height = 28.dp)
    Spacer(Modifier.height(8.dp))
    Skeleton(width = 230.dp, height = 14.dp)
    Spacer(Modifier.height(20.dp))
    Skeleton(Modifier.fillMaxWidth(), height = 180.dp, radius = 28.dp)
    Spacer(Modifier.height(24.dp))
    repeat(3) {
        Skeleton(Modifier.fillMaxWidth(), height = 76.dp, radius = 16.dp)
        Spacer(Modifier.height(4.dp))
    }
}
