package vitapmate.data

/** The queue's limits, read from their [Tunable]s at each event (ms). */
internal data class QueueConfig(val fetchLanes: Int, val queueDropMs: Long, val slowFetchMs: Long) {
    companion object {
        fun current() = QueueConfig(
            Settings.tunable(Tunable.FetchLanes),
            Settings.tunable(Tunable.QueueDrop) * 1000L,
            Settings.tunable(Tunable.SlowFetch) * 1000L,
        )
    }
}

/**
 * One run of [key] started in [flow] at [at] (ms); [id] orders runs, oldest first. [user]: the
 * student asked (goes first within its priority, never dropped); [hasCache]: the page has a copy
 * to keep showing if this one is dropped; [background]: the open-app sweep (lowest priority);
 * [logged]: its wait was logged as `queued`, so its start is logged too.
 */
internal data class Ticket(
    val id: Long,
    val key: String,
    val flow: String,
    val user: Boolean,
    val hasCache: Boolean,
    val background: Boolean,
    val at: Long,
    val logged: Boolean = false,
) {
    val page: String get() = key.substringBefore('/')
}

/** A key's run: waiting for a lane, holding one, or done with its last success at [Cooldown.succeededAt]. */
internal sealed interface Slot {
    /** [succeededAt]: when the key's run before this one succeeded, if one did. */
    data class Queued(val ticket: Ticket, val succeededAt: Long?) : Slot
    data class Running(val ticket: Ticket, val succeededAt: Long?) : Slot
    data class Cooldown(val succeededAt: Long) : Slot
}

internal sealed interface QueueEvent {
    /** A new run; its key has none queued or running. */
    data class Start(val ticket: Ticket) : QueueEvent

    /** The student asked for [key]'s run too: it goes ahead and is never dropped. */
    data class Ask(val key: String) : QueueEvent

    /** A screen showing [page] came ([on]) or went. */
    data class Shown(val page: String, val on: Boolean) : QueueEvent

    /** A page fetch in [flow] took [ms] and came over UDP or not. */
    data class Fetched(val flow: String, val ms: Long, val udp: Boolean) : QueueEvent

    /** Run [ticketId] of [key] ended: it [succeeded], or failed, was dropped or cancelled. */
    data class Finished(val ticketId: Long, val key: String, val flow: String, val succeeded: Boolean) : QueueEvent

    /** Sign-out: every run goes, and which succeeded is forgotten. */
    data object CancelAll : QueueEvent
}

internal sealed interface QueueEffect {
    /** The run got a lane. */
    data class Go(val ticketId: Long) : QueueEffect

    /** The run waited off screen too long and won't run ([Dropped]). */
    data class Drop(val ticketId: Long) : QueueEffect

    data class Cancel(val ticketId: Long) : QueueEffect

    /** A line for src `fetch.queue`. */
    data class Log(val msg: String, val flow: String, val fields: Map<String, String>) : QueueEffect
}

internal data class QueueStep(val queue: FetchQueue, val effects: List<QueueEffect> = emptyList())

/**
 * [SingleFlight]'s state, with no I/O: each key's [Slot], the pages on screen (with how many
 * screens show each), the last few fetch times (ms) and whether the last came over UDP, and the
 * lanes in use. [next] gives the state after an event and what to do about it; every event
 * drops stale waiting runs and starts the next while a lane is free.
 */
internal data class FetchQueue(
    val slots: Map<String, Slot> = emptyMap(),
    val onScreen: Map<String, Int> = emptyMap(),
    val took: List<Long> = emptyList(),
    val udp: Boolean = false,
    val lanes: Int = 0,
    val nextId: Long = 0,
) {
    /** [key]'s run queued or running, if one is. */
    fun active(key: String): Ticket? = when (val slot = slots[key]) {
        is Slot.Queued -> slot.ticket
        is Slot.Running -> slot.ticket
        is Slot.Cooldown, null -> null
    }

    /** Whether [key]'s last run succeeded at most [ms] before [now]. */
    fun succeededWithin(key: String, ms: Long, now: Long): Boolean {
        val at = when (val slot = slots[key]) {
            is Slot.Queued -> slot.succeededAt
            is Slot.Running -> slot.succeededAt
            is Slot.Cooldown -> slot.succeededAt
            null -> null
        }
        return at != null && now - at <= ms
    }

    fun isShown(page: String): Boolean = page in onScreen

    /** Most of the last few fetches took over [QueueConfig.slowFetchMs], or the last came over UDP. */
    fun slow(config: QueueConfig): Boolean = udp || (took.sorted().getOrNull((took.size - 1) / 2) ?: 0) > config.slowFetchMs

    /** Pages of the runs holding a lane, oldest first. */
    val runningPages: List<String>
        get() = slots.values.filterIsInstance<Slot.Running>().map { it.ticket }.sortedBy { it.id }.map { it.page }

    fun next(event: QueueEvent, now: Long, config: QueueConfig): QueueStep = when (event) {
        is QueueEvent.Start -> {
            val t = event.ticket
            val before = (slots[t.key] as? Slot.Cooldown)?.succeededAt
            val step = copy(slots = slots + (t.key to Slot.Queued(t, before)), nextId = maxOf(nextId, t.id + 1)).pump(t.flow, now, config)
            val q = step.queue
            val queued = q.slots[t.key] as? Slot.Queued
            if (queued == null || queued.ticket.id != t.id) step
            else QueueStep(
                q.copy(slots = q.slots + (t.key to queued.copy(ticket = t.copy(logged = true)))),
                step.effects + QueueEffect.Log(
                    "queued",
                    t.flow,
                    mapOf("page" to t.page, "position" to "${q.waiting().indexOfFirst { it.id == t.id } + 1}", "priority" to q.priorityName(t), "user" to "${t.user}", "lanes" to "${q.lanes}"),
                ),
            )
        }
        is QueueEvent.Ask -> {
            val next = when (val slot = slots[event.key]) {
                is Slot.Queued -> slots + (event.key to slot.copy(ticket = slot.ticket.copy(user = true)))
                is Slot.Running -> slots + (event.key to slot.copy(ticket = slot.ticket.copy(user = true)))
                is Slot.Cooldown, null -> slots
            }
            copy(slots = next).pump(active(event.key)?.flow.orEmpty(), now, config)
        }
        is QueueEvent.Shown -> {
            val n = (onScreen[event.page] ?: 0) + if (event.on) 1 else -1
            copy(onScreen = if (n <= 0) onScreen - event.page else onScreen + (event.page to n)).pump("", now, config)
        }
        is QueueEvent.Fetched -> copy(took = (took + event.ms).takeLast(RECENT), udp = event.udp).pump(event.flow, now, config)
        is QueueEvent.Finished -> {
            val slot = slots[event.key]
            val mine = active(event.key)?.id == event.ticketId
            val next = when {
                !mine -> slots
                event.succeeded -> slots + (event.key to Slot.Cooldown(now))
                else -> when (val before = (slot as? Slot.Running)?.succeededAt ?: (slot as? Slot.Queued)?.succeededAt) {
                    null -> slots - event.key
                    else -> slots + (event.key to Slot.Cooldown(before))
                }
            }
            copy(slots = next).pump(event.flow, now, config)
        }
        QueueEvent.CancelAll -> QueueStep(
            copy(slots = emptyMap()),
            slots.keys.mapNotNull { active(it) }.sortedBy { it.id }.map { QueueEffect.Cancel(it.id) },
        )
    }

    /** Waiting runs, the one to start next first. */
    private fun waiting(): List<Ticket> =
        slots.values.filterIsInstance<Slot.Queued>().map { it.ticket }.sortedWith(compareBy({ rank(it) }, { !it.user }, { it.id }))

    private fun rank(t: Ticket) = when {
        t.background -> 2
        t.page in onScreen -> 0
        else -> 1
    }

    private fun priorityName(t: Ticket) = listOf("on_screen", "left", "background")[rank(t)]

    /** Drops stale waiting runs, then starts the next while a lane is free. */
    private fun pump(flow: String, now: Long, config: QueueConfig): QueueStep {
        val effects = mutableListOf<QueueEffect>()
        val want = if (slow(config)) 1 else config.fetchLanes
        if (want != lanes && lanes != 0) {
            effects += QueueEffect.Log("lanes", flow, mapOf("from" to "$lanes", "to" to "$want", "udp" to "$udp", "recent_ms" to took.joinToString(",")))
        }
        val next = slots.toMutableMap()
        for (t in waiting()) {
            if (t.user || !t.hasCache || t.page in onScreen || now - t.at <= config.queueDropMs) continue
            when (val before = (next[t.key] as? Slot.Queued)?.succeededAt) {
                null -> next.remove(t.key)
                else -> next[t.key] = Slot.Cooldown(before)
            }
            effects += QueueEffect.Drop(t.id)
            effects += QueueEffect.Log("dropped", t.flow, mapOf("page" to t.page, "waited_ms" to "${now - t.at}"))
        }
        var q = copy(slots = next, lanes = want)
        while (q.slots.values.count { it is Slot.Running } < want) {
            val t = q.waiting().firstOrNull() ?: break
            val queued = q.slots.getValue(t.key) as Slot.Queued
            q = q.copy(slots = q.slots + (t.key to Slot.Running(t, queued.succeededAt)))
            effects += QueueEffect.Go(t.id)
            if (t.logged) effects += QueueEffect.Log("started", t.flow, mapOf("page" to t.page, "waited_ms" to "${now - t.at}", "priority" to q.priorityName(t)))
        }
        return QueueStep(q, effects)
    }

    private companion object {
        /** How many recent fetch times decide [slow]. */
        const val RECENT = 3
    }
}
