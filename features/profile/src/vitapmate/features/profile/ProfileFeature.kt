package vitapmate.features.profile

import com.composables.icons.lucide.CircleUser
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Feature

/** The student's VTOP profile and their mentor; the avatar photo is chosen here. */
val profileFeature = Feature("profile", "Profile", Lucide.CircleUser) { ProfileScreen() }
