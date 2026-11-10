package vitapmate

import android.content.ComponentName
import android.content.Context
import android.content.pm.PackageManager
import androidx.compose.ui.graphics.Color

/**
 * The launcher icons (Settings → Appearance → App icon): the mascot on a flat colour. Each is an
 * `activity-alias` of [MainActivity] in the manifest (`vitapmate.Icon<name>`); exactly one is
 * enabled, and Android's own record of that is the setting. The launcher may take a few seconds
 * to redraw, and moves the icon to a free spot on some phones.
 */
enum class AppIcon(val background: Color, val lightHead: Boolean) {
    Yellow(Color(0xFFFFD60A), false),
    Orange(Color(0xFFFF5A1F), true),
    Green(Color(0xFF00B869), true),
    Blue(Color(0xFF1A73E8), true),
    White(Color(0xFFFFFFFF), false),
    ;

    private fun component(context: Context) = ComponentName(context, "vitapmate.Icon$name")

    companion object {
        /** The enabled icon (the manifest enables [Yellow]). */
        fun current(context: Context): AppIcon = entries.firstOrNull {
            context.packageManager.getComponentEnabledSetting(it.component(context)) == PackageManager.COMPONENT_ENABLED_STATE_ENABLED
        } ?: Yellow

        /** Enables [icon]'s launcher entry and disables the others, without closing the app. */
        fun set(context: Context, icon: AppIcon) {
            // The new one first, so the app always has a launcher entry.
            (listOf(icon) + (entries - icon)).forEach {
                val state = if (it == icon) PackageManager.COMPONENT_ENABLED_STATE_ENABLED else PackageManager.COMPONENT_ENABLED_STATE_DISABLED
                context.packageManager.setComponentEnabledSetting(it.component(context), state, PackageManager.DONT_KILL_APP)
            }
        }
    }
}
