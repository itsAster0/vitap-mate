package vitapmate.data

import kotlin.test.AfterTest
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue
import kotlinx.coroutines.async
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import vitapmate.core.CookieAnswer
import vitapmate.core.CookieKind
import vitapmate.core.CookiePayload
import vitapmate.core.RegnoAnswer
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
        val first = async { CookieConsent.ask("f1", CookieKind.ROUTE, CookiePayload.COOKIE, "Fetch attendance", "vmate.aster0.dev") }
        val second = async { CookieConsent.ask("f2", CookieKind.ROUTE, CookiePayload.COOKIE, "Fetch attendance", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.isEmpty()) yield()
        yield()
        assertEquals(1, CookieConsent.pending.value.size)
        val yes = CookieAnswer.Allow(remember = false)
        CookieConsent.answer(CookieConsent.pending.value.single(), yes)
        assertEquals(yes, first.await())
        assertEquals(yes, second.await())
        assertTrue(CookieConsent.pending.value.isEmpty())
    }

    @Test
    fun payloadStaysSeparateWhenCoalescing() = runBlocking {
        Foreground.started()
        val cookieOnly = async { CookieConsent.ask("f1", CookieKind.ACCOUNT, CookiePayload.COOKIE, "Save", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.isEmpty()) yield()
        val withPassword = async { CookieConsent.ask("f2", CookieKind.ACCOUNT, CookiePayload.COOKIE_AND_PASSWORD, "Save", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.size < 2) yield()
        assertEquals(CookiePayload.COOKIE, CookieConsent.pending.value[0].payload)
        assertEquals(CookiePayload.COOKIE_AND_PASSWORD, CookieConsent.pending.value[1].payload)
        CookieConsent.answer(CookieConsent.pending.value.first(), CookieAnswer.Deny)
        CookieConsent.answer(CookieConsent.pending.value.single(), CookieAnswer.Deny)
        assertEquals(CookieAnswer.Deny, cookieOnly.await())
        assertEquals(CookieAnswer.Deny, withPassword.await())
    }

    @Test
    fun timeoutAnswersCancel() = runBlocking {
        val wait = CookieConsent.waitMs
        CookieConsent.waitMs = { 10L }
        try {
            val answer = CookieConsent.ask("f", CookieKind.ACCOUNT, CookiePayload.COOKIE, "Load connected apps", "vmate.aster0.dev")
            assertEquals(CookieAnswer.Deny, answer)
            assertTrue(CookieConsent.pending.value.isEmpty())
        } finally {
            CookieConsent.waitMs = wait
        }
    }

    @Test
    fun leavingTheAppAnswersCancel() = runBlocking {
        Foreground.started()
        val asked = async { CookieConsent.ask("f", CookieKind.HANDOVER, CookiePayload.COOKIE, "Give a connected app your VTOP session", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.isEmpty()) yield()
        Foreground.stopped()
        assertEquals(CookieAnswer.Deny, asked.await())
        assertTrue(CookieConsent.pending.value.isEmpty())
    }

    @Test
    fun anotherActionNeverRidesOnAShownDialog() = runBlocking {
        Foreground.started()
        val load = async { CookieConsent.ask("f1", CookieKind.ACCOUNT, CookiePayload.COOKIE, "Load your connected apps", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.isEmpty()) yield()
        val delete = async { CookieConsent.ask("f2", CookieKind.ACCOUNT, CookiePayload.COOKIE, "Delete your account on the server", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.size < 2) yield()
        val yes = CookieAnswer.Allow(remember = false)
        CookieConsent.answer(CookieConsent.pending.value.first(), yes)
        assertEquals(yes, load.await())
        assertEquals("Delete your account on the server", CookieConsent.pending.value.single().action)
        CookieConsent.answer(CookieConsent.pending.value.single(), CookieAnswer.Deny)
        assertEquals(CookieAnswer.Deny, delete.await())
    }

    @Test
    fun cancelledAskTakesItsDialogAway() = runBlocking {
        Foreground.started()
        val asked = launch { CookieConsent.ask("f", CookieKind.ACCOUNT, CookiePayload.COOKIE, "Load your connected apps", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.isEmpty()) yield()
        asked.cancelAndJoin()
        assertTrue(CookieConsent.pending.value.isEmpty())
    }

    @Test
    fun cancelledAskKeepsADialogOthersWaitOn() = runBlocking {
        Foreground.started()
        val first = launch { CookieConsent.ask("f1", CookieKind.ROUTE, CookiePayload.COOKIE, "Fetch attendance", "vmate.aster0.dev") }
        val second = async { CookieConsent.ask("f2", CookieKind.ROUTE, CookiePayload.COOKIE, "Fetch attendance", "vmate.aster0.dev") }
        while (CookieConsent.pending.value.isEmpty()) yield()
        yield()
        first.cancelAndJoin()
        val yes = CookieAnswer.Allow(remember = false)
        CookieConsent.answer(CookieConsent.pending.value.single(), yes)
        assertEquals(yes, second.await())
    }

    @Test
    fun askWhileNotShownAnswersCancelAtOnce() = runBlocking {
        val answer = CookieConsent.ask("f", CookieKind.ACCOUNT, CookiePayload.COOKIE, "Load your connected apps", "vmate.aster0.dev")
        assertEquals(CookieAnswer.Deny, answer)
        assertTrue(CookieConsent.pending.value.isEmpty())
    }

    // The registration number's sheet ([RegnoConsent]), asked through the same gate.

    @Test
    fun regnoAnswerGoesBackAndTheSheetGoes() = runBlocking {
        Foreground.started()
        val asked = async { CookieConsent.askRegno("f") }
        while (RegnoConsent.pending.value == null) yield()
        assertEquals("f", RegnoConsent.pending.value?.flow)
        RegnoConsent.answer(checkNotNull(RegnoConsent.pending.value), RegnoAnswer.SHARE)
        assertEquals(RegnoAnswer.SHARE, asked.await())
        assertNull(RegnoConsent.pending.value)
    }

    @Test
    fun regnoNotShownIsDismissedAtOnce() = runBlocking {
        assertEquals(RegnoAnswer.DISMISSED, RegnoConsent.ask("f"))
        assertNull(RegnoConsent.pending.value)
    }

    @Test
    fun regnoTimeoutIsDismissed() = runBlocking {
        Foreground.started()
        val wait = RegnoConsent.waitMs
        RegnoConsent.waitMs = { 10L }
        try {
            assertEquals(RegnoAnswer.DISMISSED, RegnoConsent.ask("f"))
            assertNull(RegnoConsent.pending.value)
        } finally {
            RegnoConsent.waitMs = wait
        }
    }

    @Test
    fun regnoLeavingTheAppIsDismissed() = runBlocking {
        Foreground.started()
        val asked = async { RegnoConsent.ask("f") }
        while (RegnoConsent.pending.value == null) yield()
        Foreground.stopped()
        assertEquals(RegnoAnswer.DISMISSED, asked.await())
        assertNull(RegnoConsent.pending.value)
    }

    @Test
    fun regnoCancelledAskTakesItsSheetAway() = runBlocking {
        Foreground.started()
        val asked = launch { RegnoConsent.ask("f") }
        while (RegnoConsent.pending.value == null) yield()
        asked.cancelAndJoin()
        assertNull(RegnoConsent.pending.value)
    }

    @Test
    fun regnoNewerAskClosesTheOlderSheet() = runBlocking {
        Foreground.started()
        val first = async { RegnoConsent.ask("f1") }
        while (RegnoConsent.pending.value == null) yield()
        val second = async { RegnoConsent.ask("f2") }
        while (RegnoConsent.pending.value?.flow != "f2") yield()
        assertEquals(RegnoAnswer.DISMISSED, first.await())
        RegnoConsent.answer(checkNotNull(RegnoConsent.pending.value), RegnoAnswer.NOT_NOW)
        assertEquals(RegnoAnswer.NOT_NOW, second.await())
    }
}
