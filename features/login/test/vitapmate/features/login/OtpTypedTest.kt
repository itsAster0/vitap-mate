package vitapmate.features.login

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue
import vitapmate.data.OtpSource

class OtpTypedTest {
    @Test
    fun aTypedCodeIsNotedUnlessAutoFilledOrGmailIsThePick() {
        assertTrue(typedForForwarding("123456", emptySet(), OtpSource.Forwarding))
        // Forwarding's status needn't have loaded: Rust notes it only with an address set up.
        assertTrue(typedForForwarding("123456", emptySet(), null))
        assertFalse(typedForForwarding("123456", setOf("123456"), OtpSource.Forwarding))
        assertFalse(typedForForwarding("123456", emptySet(), OtpSource.Gmail))
    }
}
