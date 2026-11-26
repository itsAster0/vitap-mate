package vitapmate.features.login

import androidx.compose.runtime.saveable.SaverScope
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull

class PhonesConfirmTest {
    private val scope = SaverScope { true }

    private fun roundtrip(c: PhonesConfirm?): PhonesConfirm? =
        with(PhonesConfirm.Saver) { scope.save(c)?.let(::restore) }

    @Test
    fun anOpenConfirmSurvivesProcessDeath() {
        assertEquals(PhonesConfirm.Delete, roundtrip(PhonesConfirm.Delete))
        assertEquals(PhonesConfirm.Remove("ab:cd"), roundtrip(PhonesConfirm.Remove("ab:cd")))
        assertNull(roundtrip(null))
        assertNull(PhonesConfirm.Saver.restore("other"))
    }
}
