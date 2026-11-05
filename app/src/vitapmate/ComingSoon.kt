package vitapmate

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import com.composables.icons.lucide.Construction
import com.composables.icons.lucide.Lucide
import vitapmate.ui.EmptyView

/** A feature that isn't built yet. */
@Composable
fun ComingSoon(id: String) {
    val feature = featureById(id)
    Box(Modifier.fillMaxSize(), Alignment.Center) {
        EmptyView(feature?.icon ?: Lucide.Construction, "Coming soon", message = "${feature?.label ?: id} isn't available yet.")
    }
}
