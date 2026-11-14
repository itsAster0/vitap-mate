package vitapmate.data

import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import kotlinx.coroutines.async
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import vitapmate.core.CookieAnswer
import vitapmate.core.CookieKind
import vitapmate.observability.Unattended

class CookieConsentTest {
    @AfterTest
    fun cleanUp() {
        Foreground.stopped()
    }

    @Test
    fun unattendedFlowIsNotAttended() {
        Foreground.started()
        assertTrue(CookieConsent.attended("f"))
        // As `flow(name, unattended = true)` marks its id while it runs (androidx.tracing needs a device).
        Unattended.ids += "f"
        try {
            assertFalse(CookieConsent.attended("f"))
        } finally {
            Unattended.ids -= "f"
        }
    }

    @Test
    fun notShownIsNotAttended() {
        Foreground.started()
        Foreground.stopped()
        assertFalse(CookieConsent.attended("f"))
    }

    @Test
    fun twoAsksOfOneActionShareOneDialog() = runBlocking {
        Foreground.started()
        val first = async { CookieConsent.ask("f1", CookieKind.ROUTE, "Fetch attendance", "vmate.aster0.dev") }
        val second = async { CookieConsent.ask("f2", CookieKind.ROUTE, "Fetch attendance", "vmate.aster0.dev") }
        while (CookieConsent.pending.isEmpty()) yield()
        yield()
        assertEquals(1, CookieConsent.pending.size)
        val yes = CookieAnswer(send = true, remember = false)
        CookieConsent.answer(CookieConsent.pending.single(), yes)
        assertEquals(yes, first.await())
        assertEquals(yes, second.await())
        assertTrue(CookieConsent.pending.isEmpty())
    }

    @Test
    fun timeoutAnswersCancel() = runBlocking {
        val wait = CookieConsent.waitMs
        CookieConsent.waitMs = { 10L }
        try {
            val answer = CookieConsent.ask("f", CookieKind.ACCOUNT, "Load connected apps", "vmate.aster0.dev")
            assertEquals(CookieAnswer(send = false, remember = false), answer)
            assertTrue(CookieConsent.pending.isEmpty())
        } finally {
            CookieConsent.waitMs = wait
        }
    }

    @Test
    fun leavingTheAppAnswersCancel() = runBlocking {
        Foreground.started()
        val asked = async { CookieConsent.ask("f", CookieKind.HANDOVER, "Give a connected app your VTOP session", "vmate.aster0.dev") }
        while (CookieConsent.pending.isEmpty()) yield()
        Foreground.stopped()
        assertEquals(CookieAnswer(send = false, remember = false), asked.await())
        assertTrue(CookieConsent.pending.isEmpty())
    }

    @Test
    fun anotherActionNeverRidesOnAShownDialog() = runBlocking {
        Foreground.started()
        val load = async { CookieConsent.ask("f1", CookieKind.ACCOUNT, "Load your connected apps", "vmate.aster0.dev") }
        while (CookieConsent.pending.isEmpty()) yield()
        val delete = async { CookieConsent.ask("f2", CookieKind.ACCOUNT, "Delete your account on the server", "vmate.aster0.dev") }
        while (CookieConsent.pending.size < 2) yield()
        val yes = CookieAnswer(send = true, remember = false)
        CookieConsent.answer(CookieConsent.pending.first(), yes)
        assertEquals(yes, load.await())
        assertEquals("Delete your account on the server", CookieConsent.pending.single().action)
        CookieConsent.answer(CookieConsent.pending.single(), CookieAnswer(send = false, remember = false))
        assertFalse(delete.await().send)
    }

    @Test
    fun cancelledAskTakesItsDialogAway() = runBlocking {
        Foreground.started()
        val asked = launch { CookieConsent.ask("f", CookieKind.ACCOUNT, "Load your connected apps", "vmate.aster0.dev") }
        while (CookieConsent.pending.isEmpty()) yield()
        asked.cancelAndJoin()
        assertTrue(CookieConsent.pending.isEmpty())
    }

    @Test
    fun cancelledAskKeepsADialogOthersWaitOn() = runBlocking {
        Foreground.started()
        val first = launch { CookieConsent.ask("f1", CookieKind.ROUTE, "Fetch attendance", "vmate.aster0.dev") }
        val second = async { CookieConsent.ask("f2", CookieKind.ROUTE, "Fetch attendance", "vmate.aster0.dev") }
        while (CookieConsent.pending.isEmpty()) yield()
        yield()
        first.cancelAndJoin()
        val yes = CookieAnswer(send = true, remember = false)
        CookieConsent.answer(CookieConsent.pending.single(), yes)
        assertEquals(yes, second.await())
    }

    @Test
    fun askWhileNotShownAnswersCancelAtOnce() = runBlocking {
        val answer = CookieConsent.ask("f", CookieKind.ACCOUNT, "Load your connected apps", "vmate.aster0.dev")
        assertFalse(answer.send)
        assertTrue(CookieConsent.pending.isEmpty())
    }
}
