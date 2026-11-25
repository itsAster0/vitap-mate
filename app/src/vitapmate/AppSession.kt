package vitapmate

/**
 * Where the app is between start and the shell ([App]): one phase at a time, moved only by
 * [next]. Who is signed in is Rust's ([vitapmate.data.AccountWatch]); [user] is their username.
 */
sealed interface AppSession {
    /** The core is starting. */
    data object Booting : AppSession

    /** The core didn't start; [error] shows instead of the app. */
    data class BootFailed(val error: Throwable) : AppSession

    data object SignedOut : AppSession

    /** Signed in: the saved semester pick is being read (after the last student's things went). */
    data class LoadingAccount(val user: String) : AppSession

    /** No semester picked; [firstTime]: the setup page ([Onboarding]) follows the pick. */
    data class PickingSemester(val user: String, val firstTime: Boolean) : AppSession

    data class Onboarding(val user: String) : AppSession

    data class Ready(val user: String) : AppSession
}

/** How far this install got with the setup page after the first semester pick. */
enum class Setup {
    /** No semester was picked here yet: the setup page follows the first pick. */
    NotYet,

    /** Picked, and the setup page is not done (the app closed on it): it opens again. */
    Pending,
    Done,
}

/** What moves [AppSession] on. */
sealed interface SessionEvent {
    data class BootError(val error: Throwable) : SessionEvent

    /** Rust said who is signed in: [user], or nobody (null). */
    data class Account(val user: String?) : SessionEvent

    /** The saved pick was read: [picked] a semester. */
    data class AccountRead(val picked: Boolean, val setup: Setup) : SessionEvent

    /** The semester pick came or went (picked in the sheet, or forgotten). */
    data class PickChanged(val picked: Boolean, val setup: Setup) : SessionEvent

    data object OnboardingDone : SessionEvent
}

/** The signed-in username, for the avatar; null signed out. */
val AppSession.user: String?
    get() = when (this) {
        AppSession.Booting, is AppSession.BootFailed, AppSession.SignedOut -> null
        is AppSession.LoadingAccount -> user
        is AppSession.PickingSemester -> user
        is AppSession.Onboarding -> user
        is AppSession.Ready -> user
    }

/** Whether a student is signed in (the account may still be loading). */
val AppSession.signedIn: Boolean
    get() = user != null

/** [s] after [e]; an event that doesn't apply leaves it as it is. */
fun next(s: AppSession, e: SessionEvent): AppSession = when (e) {
    is SessionEvent.BootError -> if (s == AppSession.Booting) AppSession.BootFailed(e.error) else s
    is SessionEvent.Account -> when {
        s is AppSession.BootFailed -> s
        e.user == null -> AppSession.SignedOut
        // The same student (their status changed): where they are stays.
        s.user.equals(e.user, ignoreCase = true) -> s
        // Signed in, or another student over the last one: their pick is read again.
        else -> AppSession.LoadingAccount(e.user)
    }
    is SessionEvent.AccountRead -> when (s) {
        is AppSession.LoadingAccount -> when {
            !e.picked -> AppSession.PickingSemester(s.user, firstTime = e.setup != Setup.Done)
            e.setup == Setup.Pending -> AppSession.Onboarding(s.user)
            else -> AppSession.Ready(s.user)
        }
        else -> s
    }
    is SessionEvent.PickChanged -> when {
        s is AppSession.PickingSemester && e.picked -> if (s.firstTime) AppSession.Onboarding(s.user) else AppSession.Ready(s.user)
        s is AppSession.Ready && !e.picked -> AppSession.PickingSemester(s.user, firstTime = e.setup != Setup.Done)
        else -> s
    }
    SessionEvent.OnboardingDone -> if (s is AppSession.Onboarding) AppSession.Ready(s.user) else s
}
