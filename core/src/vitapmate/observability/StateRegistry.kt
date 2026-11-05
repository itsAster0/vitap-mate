package vitapmate.observability

import java.util.concurrent.ConcurrentHashMap

/**
 * What the app shows now, for the agent state dump (`mise run state`): screens and the app
 * report values under a key, the dump writes them all. Keys: `route` (the app: `route`, tabs,
 * open sheets), `page.<id>` (every [vitapmate.data.rememberPage], automatically), anything else
 * a screen wants an agent to see. Values: strings, numbers, booleans, null, lists and maps.
 * Never secrets.
 */
object StateRegistry {
    private val values = ConcurrentHashMap<String, Map<String, Any?>>()

    fun report(key: String, value: Map<String, Any?>) {
        values[key] = value
    }

    fun remove(key: String) {
        values.remove(key)
    }

    fun snapshot(): Map<String, Map<String, Any?>> = values.toSortedMap()
}
