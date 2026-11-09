package vitapmate.observability

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class RunsTest {
    private fun runs(text: String, since: Long = 0) = runsOf(groupFlows(text.trimIndent().lines().mapNotNull(::parseLogLine)), since)

    private fun Run.chain() = steps.joinToString(" → ") { s -> s.label + (s.reason?.let { " ($it)" } ?: "") + " " + s.state }

    @Test
    fun routeAndFallback() {
        val r = runs(
            """
            {"ev":"start","flow":"a1","l":"info","msg":"attendance.refresh start","src":"flow","step":"attendance.refresh","t":1000}
            {"flow":"a1","l":"warn","msg":"route failed; skipped for 5 min","reason":"edge answered 502","route":"Proxy","src":"rust.route","t":1010}
            {"flow":"a1","l":"info","msg":"route used","route":"Direct","src":"rust.route","t":1100}
            {"flow":"a1","l":"info","msg":"response vtop.vitap.ac.in/vtop/processViewStudentAttendance 200 90ms","src":"rust.network","t":1100}
            {"flow":"a1","l":"info","ms":100,"msg":"done","outcome":"ok","src":"core.fetch_attendance","t":1105}
            {"ev":"done","flow":"a1","l":"info","ms":"110","msg":"attendance.refresh done","outcome":"ok","src":"flow","step":"attendance.refresh","t":1110}
            """,
        ).single()
        assertEquals("Direct", r.route)
        assertEquals("Proxy failed → Direct", r.fallback)
        val udp = parseLogLine("""{"flow":"b","l":"info","msg":"route used","route":"Udp","src":"rust.route","t":1}""")!!
        assertEquals("UDP", routeOf(listOf(udp)))
        assertEquals(null, fallbackOf(listOf(udp)))
    }

    @Test
    fun signedInRefreshIsOneStep() {
        // Real lines from the emulator.
        val r = runs(
            """
            {"ev":"start","flow":"71eddc72","l":"info","msg":"attendance.refresh start","src":"flow","step":"attendance.refresh","t":1000}
            {"age_s":"2","count":"11","flow":"71eddc72","kind":"Ready","l":"info","ms":"1945","msg":"refreshing","src":"ui.attendance","state":"refreshing","t":1002,"trigger":"pull"}
            {"flow":"71eddc72","l":"info","msg":"request get_attendance.send POST https://vtop.vitap.ac.in/vtop/processViewStudentAttendance","src":"rust.network","t":1040}
            {"flow":"71eddc72","l":"info","msg":"response vtop.vitap.ac.in/vtop/processViewStudentAttendance 200 158ms","src":"rust.network","t":1199}
            {"courses":11,"flow":"71eddc72","l":"info","msg":"attendance","src":"core.fetch_attendance","t":1205}
            {"flow":"71eddc72","l":"info","ms":164,"msg":"done","outcome":"ok","src":"core.fetch_attendance","t":1205}
            {"flow":"71eddc72","l":"info","msg":"fresh","src":"ui.attendance","state":"fresh","t":1269}
            {"ev":"done","flow":"71eddc72","l":"info","ms":"270","msg":"attendance.refresh done","outcome":"ok","src":"flow","step":"attendance.refresh","t":1270}
            """,
        ).single()
        assertEquals("Attendance", r.title)
        assertEquals(RunTrigger.Pulled, r.trigger)
        assertEquals("Attendance Done", r.chain())
        assertEquals(listOf(PageHit("Attendance", "200", 158)), r.steps.single().pages)
        assertEquals(1002L, r.steps.single().startT)
    }

    @Test
    fun expiredSessionSignsInWaitsForOtpThenFetches() {
        val list = runs(
            """
            {"ev":"start","flow":"a","msg":"timetable.refresh start","src":"flow","step":"timetable.refresh","t":1000}
            {"flow":"a","msg":"refreshing","src":"ui.timetable","state":"refreshing","t":1001,"trigger":"open"}
            {"flow":"a","msg":"session expired, signing in again","src":"core.fetch_timetable","t":1100}
            {"flow":"a","msg":"auth.flow#1 start login requested","src":"rust.auth","t":1101}
            {"flow":"a","msg":"auth.flow#1 captcha captcha answer was rejected, retrying login","src":"rust.auth","t":1500}
            {"flow":"a","msg":"response vtop.vitap.ac.in/vtop/login 200 108ms","src":"rust.network","t":1600}
            {"flow":"a","msg":"auth.flow#1 otp security OTP verification is required","src":"rust.auth","t":2000}
            {"flow":"a","ms":1000,"msg":"done","outcome":"OtpRequired","src":"core.fetch_timetable","t":2001}
            {"ev":"failed","flow":"a","msg":"timetable.refresh failed: OtpRequired","outcome":"OtpRequired","src":"flow","step":"timetable.refresh","t":2002}
            {"ev":"start","flow":"b","msg":"fetch_otp.submit_otp start","src":"flow","step":"fetch_otp.submit_otp","t":9000}
            {"flow":"b","ms":500,"msg":"done","outcome":"ok","src":"core.submit_otp","t":9500}
            {"ev":"done","flow":"b","msg":"fetch_otp.submit_otp done","outcome":"ok","src":"flow","step":"fetch_otp.submit_otp","t":9501}
            {"ev":"start","flow":"c","msg":"timetable.refresh start","src":"flow","step":"timetable.refresh","t":9600}
            {"flow":"c","msg":"refreshing","src":"ui.timetable","state":"refreshing","t":9601,"trigger":"otp"}
            {"flow":"c","msg":"response vtop.vitap.ac.in/vtop/processViewTimeTable 200 90ms","src":"rust.network","t":9700}
            {"flow":"c","ms":100,"msg":"done","outcome":"ok","src":"core.fetch_timetable","t":9701}
            {"ev":"done","flow":"c","msg":"timetable.refresh done","outcome":"ok","src":"flow","step":"timetable.refresh","t":9702}
            """,
        )
        val r = list.single()
        assertEquals(listOf("a", "c"), r.flows)
        assertEquals(RunTrigger.Opened, r.trigger)
        assertEquals("Timetable (Not signed in) Skipped → Sign in (2 tries) Done → OTP Done → Timetable Done", r.chain())
        assertEquals(7501L, r.steps[2].ms)
    }

    @Test
    fun otpStillWaitingUnlessCancelledOrGone() {
        val text = """
            {"ev":"start","flow":"a","msg":"marks.refresh start","src":"flow","step":"marks.refresh","t":1000}
            {"flow":"a","ms":1,"msg":"done","outcome":"OtpRequired","src":"core.fetch_marks","t":1001}
            {"ev":"failed","flow":"a","msg":"marks.refresh failed","outcome":"OtpRequired","src":"flow","step":"marks.refresh","t":1002}
        """
        val r = runs(text).single()
        assertTrue(r.waiting)
        assertEquals("Marks (Not signed in) Skipped → OTP WaitingOtp", r.chain())
        assertTrue(listOf(r).settleOtp(pending = true, since = 0).single().waiting)
        assertEquals("OTP (Not entered) Skipped", listOf(r).settleOtp(pending = false, since = 0).single().steps.last().let { "${it.label} (${it.reason}) ${it.state}" })
        val cancelled = runs(
            text + """
            {"ev":"start","flow":"b","msg":"otp.cancel start","src":"flow","step":"otp.cancel","t":3000}
            {"ev":"done","flow":"b","msg":"otp.cancel done","outcome":"ok","src":"flow","step":"otp.cancel","t":3001}
            """,
        ).single()
        assertEquals("OTP (Cancelled) Skipped", cancelled.steps.last().let { "${it.label} (${it.reason}) ${it.state}" })
    }

    @Test
    fun sweepFailuresLocalCallsAndCutOffFlows() {
        val list = runs(
            """
            {"ev":"start","flow":"s","msg":"sync.stale start","src":"flow","step":"sync.stale","t":1000}
            {"flow":"s","ms":0,"msg":"done","outcome":"ok","src":"core.cached_attendance","t":1001}
            {"flow":"s","msg":"response vtop.vitap.ac.in/vtop/processViewStudentAttendance 200 100ms","src":"rust.network","t":1100}
            {"flow":"s","ms":100,"msg":"done","outcome":"ok","src":"core.fetch_attendance","t":1101}
            {"flow":"s","ms":0,"msg":"done","outcome":"ok","src":"core.cached_marks","t":1102}
            {"flow":"s","msg":"request get_marks.send POST https://vtop.vitap.ac.in/vtop/x","src":"rust.network","t":1103}
            {"flow":"s","ms":30,"msg":"done","outcome":"Offline","src":"core.fetch_marks","t":1133}
            {"ev":"failed","flow":"s","msg":"sync.stale failed","outcome":"Stop","src":"flow","step":"sync.stale","t":1134}
            {"ev":"start","flow":"u","msg":"attendance.filter start","src":"flow","step":"attendance.filter","t":1200}
            {"ev":"done","flow":"u","msg":"attendance.filter done","outcome":"ok","src":"flow","step":"attendance.filter","t":1201}
            {"ev":"start","flow":"k","msg":"exams.refresh start","src":"flow","step":"exams.refresh","t":1300}
            {"flow":"k","msg":"fetching","src":"ui.exams","state":"fetching","t":1301,"trigger":"stale"}
            {"flow":"k","msg":"request get_exams.send POST https://vtop.vitap.ac.in/vtop/y","src":"rust.network","t":1302}
            """,
            since = 5000,
        )
        assertEquals(2, list.size)
        val (cut, sweep) = list
        assertEquals("Exams (Interrupted) Skipped", cut.chain())
        assertEquals(RunTrigger.Automatic, sweep.trigger)
        assertEquals("Background refresh", sweep.title)
        assertEquals("Attendance Done → Marks (No internet) Failed", sweep.chain())
        assertTrue(sweep.failed)
    }

    @Test
    fun runningStepIsLive() {
        val r = runs(
            """
            {"ev":"start","flow":"k","msg":"exams.refresh start","src":"flow","step":"exams.refresh","t":1300}
            {"flow":"k","msg":"fetching","src":"ui.exams","state":"fetching","t":1301,"trigger":"pull"}
            """,
        ).single()
        assertTrue(r.running)
        assertEquals("Exams Running", r.chain())
    }

    @Test
    fun backgroundSyncSkippedOrNotified() {
        val list = runs(
            """
            {"ev":"start","flow":"s1","l":"info","msg":"sync.background start","src":"flow","step":"sync.background","t":1000}
            {"flow":"s1","l":"info","msg":"skipped: OTP auto-fill isn't set up","src":"sync","t":1001}
            {"ev":"done","flow":"s1","l":"info","ms":"2","msg":"sync.background done","outcome":"ok","src":"flow","step":"sync.background","t":1002}
            {"ev":"start","flow":"s2","l":"info","msg":"sync.background start","src":"flow","step":"sync.background","t":2000}
            {"ev":"start","flow":"p2","l":"info","msg":"sync.background.pages start","src":"flow","step":"sync.background.pages","t":2001}
            {"flow":"p2","l":"info","msg":"response vtop.vitap.ac.in/vtop/processViewStudentAttendance 200 90ms","src":"rust.network","t":2100}
            {"flow":"p2","l":"info","ms":100,"msg":"done","outcome":"ok","src":"core.fetch_attendance","t":2105}
            {"ev":"done","flow":"p2","l":"info","ms":"110","msg":"sync.background.pages done","outcome":"ok","src":"flow","step":"sync.background.pages","t":2110}
            {"flow":"s2","l":"info","msg":"background done","notified":"2","src":"sync","t":2200}
            {"ev":"done","flow":"s2","l":"info","ms":"201","msg":"sync.background done","outcome":"ok","src":"flow","step":"sync.background","t":2201}
            """,
        )
        assertEquals(2, list.size)
        assertEquals("Background sync", list[0].title)
        assertEquals("Attendance Done → 2 changes notified Done", list[0].chain())
        assertEquals("Background sync (OTP auto-fill isn't set up) Skipped", list[1].chain())
    }
}
