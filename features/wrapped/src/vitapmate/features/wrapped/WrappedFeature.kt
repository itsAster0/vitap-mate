package vitapmate.features.wrapped

import com.composables.icons.lucide.Lucide
import com.composables.icons.lucide.Sparkles
import vitapmate.ui.Feature

/** Wrapped's route: More's card opens it with `nav.push(Route.Screen(WRAPPED))`. */
const val WRAPPED = "wrapped"

/**
 * The selected semester's story, once its classes are over: one fact per slide, a summary, and
 * a share image of each. Full screen and in no nav group, so it is never a tab or a More tile.
 */
val wrappedFeature = Feature(WRAPPED, "Wrapped", Lucide.Sparkles, fullScreen = true) { WrappedScreen() }
