package vitapmate

import android.app.PendingIntent
import android.content.Intent
import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.text.format.DateFormat
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import vitapmate.data.AppCore
import vitapmate.data.AppScope
import vitapmate.data.Settings
import vitapmate.features.timetable.istDay
import vitapmate.features.timetable.istMinute
import vitapmate.features.timetable.meeting
import vitapmate.observability.Log
import vitapmate.observability.flow
import vitapmate.ui.clock

/**
 * Quick Settings tile for In class ([ClassMode]): active while it is on, with the class on now or
 * the next one today from the cached timetable. A tap switches it as the in-app switch does, and
 * opens Android's Do Not Disturb access when the app doesn't have it yet. Flow `tile.class`.
 */
class ClassTile : TileService() {
    override fun onStartListening() = update(toggle = false)

    override fun onClick() = update(toggle = true)

    private fun update(toggle: Boolean) {
        val app = applicationContext
        AppScope.launch {
            flow("tile.class") { f ->
                Settings.load(app)
                AppCore.start(app)
                if (toggle && ClassMode.supported()) {
                    val on = !Settings.classMode
                    Settings.classMode = on
                    Log.info("classmode", "tile", f, mapOf("on" to "$on"))
                    if (on && !ClassMode.granted(app)) allow() else ClassMode.refresh(app, "classmode.tile")
                }
                val next = runCatching { withContext(Dispatchers.IO) { next(f) } }
                    .onFailure { Log.warn("classmode", "tile: timetable unavailable: ${it::class.simpleName}", f) }.getOrNull()
                val tile = qsTile ?: return@flow
                tile.label = "In class"
                tile.state = when {
                    !ClassMode.supported() -> Tile.STATE_UNAVAILABLE
                    Settings.classMode -> Tile.STATE_ACTIVE
                    else -> Tile.STATE_INACTIVE
                }
                if (Build.VERSION.SDK_INT >= 29) tile.subtitle = next ?: "No more classes today"
                tile.updateTile()
            }
        }
    }

    /** The class on now ("… · until 10:50") or the next one today ("… · 11:00"), or null. */
    private suspend fun next(f: String): String? {
        val c = AppCore.await()
        val ms = System.currentTimeMillis()
        val now = istMinute(ms)
        val day = c.cachedTimetable(f)?.let { c.timetableDays(f, istDay(ms), 1).firstOrNull() } ?: return null
        val s = meeting(day).map { it.slot }.sortedBy { it.start }.firstOrNull { it.end > now } ?: return null
        val h24 = DateFormat.is24HourFormat(applicationContext)
        val time = if (s.start <= now) "until ${clock(s.end, h24)}" else clock(s.start, h24)
        return listOf(s.name, listOf(s.block, s.room).filter { it.isNotEmpty() }.joinToString(" "), time).filter { it.isNotEmpty() }.joinToString(" · ")
    }

    /** Android's Do Not Disturb access, with the shade collapsed. */
    private fun allow() {
        val intent = Intent(android.provider.Settings.ACTION_NOTIFICATION_POLICY_ACCESS_SETTINGS).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        if (Build.VERSION.SDK_INT >= 34) startActivityAndCollapse(PendingIntent.getActivity(this, 0, intent, PendingIntent.FLAG_IMMUTABLE))
        else @Suppress("DEPRECATION") startActivityAndCollapse(intent)
    }
}
