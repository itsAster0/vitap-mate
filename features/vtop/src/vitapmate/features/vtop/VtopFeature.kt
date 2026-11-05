package vitapmate.features.vtop

import com.composables.icons.lucide.Globe
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Feature

/** VTOP itself in a web view, signed in with the app's session (the Flutter app's "Open VTOP"). */
val vtopFeature = Feature("vtop", "VTOP", Lucide.Globe, fullScreen = true) { VtopScreen() }
