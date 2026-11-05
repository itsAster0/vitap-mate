package vitapmate.features.outing

import com.composables.icons.lucide.DoorOpen
import com.composables.icons.lucide.Lucide
import vitapmate.ui.Feature

/**
 * Hostel outings: general and weekend requests with their status, applying, cancelling, and the
 * pass (a detail: `<general|weekend>:<pass id>`).
 */
val outingFeature = Feature("outing", "Outing", Lucide.DoorOpen, detail = { item -> PassScreen(item) }) { OutingScreen() }
