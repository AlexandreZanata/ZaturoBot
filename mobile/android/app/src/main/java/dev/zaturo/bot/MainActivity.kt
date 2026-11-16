package dev.zaturo.bot

import android.app.Activity
import android.app.AlertDialog
import android.app.DatePickerDialog
import android.content.Intent
import android.content.res.ColorStateList
import android.graphics.drawable.RippleDrawable
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.text.SpannableString
import android.text.Spanned
import android.text.TextUtils
import android.text.style.ForegroundColorSpan
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.view.ViewTreeObserver
import android.view.WindowInsets
import android.view.WindowManager
import android.view.accessibility.AccessibilityNodeInfo
import android.widget.*
import org.json.JSONObject
import java.time.Instant
import java.time.LocalDate
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Base64
import java.util.Locale
import java.util.UUID
import java.util.concurrent.Executors
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.math.max

/** Native owner companion. Presentation never grants permissions to a worker. */
class MainActivity : Activity() {
    private val background = Executors.newSingleThreadExecutor()
    private var requestedCaptureTask: String? = null
    private var requestedCaptureUntil=0L
    private val main = Handler(Looper.getMainLooper())
    private val busy = AtomicBoolean(false)
    private val timeline = FeedTimeline()
    private lateinit var ui: MobileUi
    private lateinit var privateSettings: PrivateSettings
    private lateinit var tunnel: SshTunnel
    private lateinit var phoneView: PrivatePhoneView
    private lateinit var container: LinearLayout
    private lateinit var navigation: LinearLayout
    private lateinit var statusRow: LinearLayout
    private lateinit var status: TextView
    private lateinit var statusDot: View
    private lateinit var feed: LinearLayout
    private lateinit var feedScroll: ScrollView
    private lateinit var draft: EditText
    private lateinit var search: EditText
    private lateinit var searchPanel: LinearLayout
    private lateinit var dayButton: Button
    private lateinit var filterButton: Button
    private var visible = false
    private var currentPage = "hub"
    private var selectedDay = LocalDate.now()
    private var filterIndex = 0
    private var queryText = ""
    private var searchDraft = ""
    private var draftText = ""
    private var searchExpanded = false
    private var scrollRevision = 0L
    private var forceEndQueued = false
    private var revealAfterFreshPage = false
    private var submissionRunning = false
    private var queuedSubmission: Pair<String, String?>? = null
    private var imeVisible = false
    private var configuration: JSONObject? = null
    private var enrollment: Enrollment? = null
    private var generation = 0L
    private val feedFilters = listOf("Tudo" to "", "Conversas" to "chat", "Tarefas" to "task",
        "Arquivos" to "file", "Android" to "android", "Serviços" to "service")
    private class Pending(val message: String, val worker: String?, val action: String?, val id: String)
    private var pending: Pending? = null
    private val poll = object : Runnable {
        override fun run() {
            if (!visible) return
            if (currentPage == "hub" && !busy.get() && atConversationEnd()) refresh(false)
            main.postDelayed(this, 10000)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_SECURE)
        if (Build.VERSION.SDK_INT >= 30) window.setDecorFitsSystemWindows(false)
        ui = MobileUi(this)
        filterIndex = (savedInstanceState?.getInt("kind") ?: 0).coerceIn(feedFilters.indices)
        selectedDay = runCatching { LocalDate.parse(savedInstanceState?.getString("day")) }.getOrDefault(LocalDate.now())
        privateSettings = PrivateSettings(this)
        configuration = privateSettings.read()
        enrollment = configuration?.let { parseEnrollmentOrNull(it) }
        tunnel = SshTunnel { connected, message ->
            main.post {
                if (visible) {
                    setStatus(message, connected)
                    if (connected) {refresh(false);if(::phoneView.isInitialized && currentPage=="hub") phoneView.resumePrepared()}
                }
            }
        }
        buildShell()
        phoneView=PrivatePhoneView(this,ui,{api()},{card ->
            if(visible && currentPage=="hub") {feed.addView(card,LinearLayout.LayoutParams(-1,-2));revealVisibleEnd()}
        },{message ->if(visible) setStatus(message)},{visible && currentPage=="hub" && hasWindowFocus()})
        showHub()
    }

    override fun onStart() {
        super.onStart()
        visible = true
        if (currentPage == "hub") revealVisibleEnd()
        enrollment?.let { tunnel.start(it) }
        if (enrollment == null) refresh(false)
        main.postDelayed(poll, 10000)
    }
    override fun onStop() {
        rememberHub()
        visible = false
        requestedCaptureTask = null
        main.removeCallbacks(poll)
        phoneView.clear()
        tunnel.stop()
        super.onStop()
    }
    override fun onDestroy() { phoneView.close(); background.shutdownNow(); super.onDestroy() }
    override fun onWindowFocusChanged(hasFocus: Boolean) {super.onWindowFocusChanged(hasFocus);if(::phoneView.isInitialized) {if(!hasFocus) phoneView.clear() else if(visible && currentPage=="hub" && (enrollment==null || tunnel.ready)) phoneView.resumePrepared()}}
    override fun onSaveInstanceState(outState: Bundle) {
        rememberHub()
        outState.putInt("kind", filterIndex)
        outState.putString("day", selectedDay.toString())
        super.onSaveInstanceState(outState)
    }

    private fun buildShell() {
        val outer = column().apply { setBackgroundColor(MobileUi.BG) }
        val header = LinearLayout(this).apply {
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(16), 0, dp(16), 0)
        }
        val mark = ImageView(this).apply {
            setImageResource(R.drawable.zaturo_mark)
            scaleType = ImageView.ScaleType.FIT_CENTER
            translationY = -dp(2).toFloat()
            importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
        }
        header.addView(mark, LinearLayout.LayoutParams(dp(52), dp(52)).apply { marginStart = -dp(6); marginEnd = dp(4) })
        val title = ui.text("ZaturoBot", 25, bold = true).apply {
            gravity = Gravity.CENTER_VERTICAL
            text = SpannableString("ZaturoBot").apply {
                setSpan(ForegroundColorSpan(MobileUi.YELLOW), 6, 9, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
            }
            if (Build.VERSION.SDK_INT >= 28) isAccessibilityHeading = true
        }
        header.addView(title, LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.MATCH_PARENT, 1f))
        outer.addView(header, LinearLayout.LayoutParams(-1, dp(56)))
        statusRow = LinearLayout(this).apply {
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(20), 0, dp(20), dp(8))
        }
        statusDot = View(this).apply { background = ui.rounded(MobileUi.MUTED, 8, MobileUi.MUTED) }
        statusRow.addView(statusDot, LinearLayout.LayoutParams(dp(6), dp(6)).apply { marginEnd = dp(8) })
        status = ui.text("Conecte seu hub privado.", 11, MobileUi.MUTED).apply {
            maxLines = 1; ellipsize = TextUtils.TruncateAt.END
            accessibilityLiveRegion = View.ACCESSIBILITY_LIVE_REGION_POLITE
        }
        statusRow.addView(status, LinearLayout.LayoutParams(0, dp(20), 1f))
        outer.addView(statusRow)
        container = column().apply { setPadding(dp(16), 0, dp(16), 0) }
        outer.addView(container, LinearLayout.LayoutParams(-1, 0, 1f))
        navigation = LinearLayout(this).apply {
            gravity = Gravity.CENTER_VERTICAL
            setPadding(dp(12), dp(8), dp(12), dp(8))
            background = ui.rounded(MobileUi.BG, 0)
        }
        val navigationHeight = (54 + 22 * resources.configuration.fontScale).toInt().coerceAtLeast(76)
        outer.addView(navigation, LinearLayout.LayoutParams(-1, dp(navigationHeight)))
        if (Build.VERSION.SDK_INT >= 30) outer.setOnApplyWindowInsetsListener { view, insets ->
            val system = insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.displayCutout())
            val keyboard = insets.getInsets(WindowInsets.Type.ime())
            val shown = insets.isVisible(WindowInsets.Type.ime())
            val keepEnd = atConversationEnd()
            view.setPadding(system.left, system.top, system.right, max(system.bottom, keyboard.bottom))
            navigation.visibility = if (shown && currentPage == "hub") View.GONE else View.VISIBLE
            statusRow.visibility = if (shown && currentPage == "hub") View.GONE else View.VISIBLE
            if (shown != imeVisible && keepEnd && ::feedScroll.isInitialized) revealVisibleEnd()
            imeVisible = shown
            insets
        }
        setContentView(outer)
    }

    private fun renderNavigation() {
        navigation.removeAllViews()
        val entries = listOf(Triple("hub", "Hub", R.drawable.ic_chat),
            Triple("services", "Serviços", R.drawable.ic_services),
            Triple("reconnect", "Reconectar", R.drawable.ic_sync),
            Triple("settings", "Ajustes", R.drawable.ic_settings))
        for ((page, label, drawable) in entries) {
            val selected = currentPage == page
            val color = if (selected) MobileUi.BG else MobileUi.MUTED
            val item = column().apply {
                gravity = Gravity.CENTER
                isClickable = true; isFocusable = true; isSelected = selected
                contentDescription = label
                elevation = 0f; stateListAnimator = null
                val fill = if (selected) MobileUi.YELLOW else MobileUi.SURFACE
                background = RippleDrawable(ColorStateList.valueOf(0x22404040),
                    ui.rounded(fill, 17, if (selected) MobileUi.YELLOW else MobileUi.STROKE), null)
                accessibilityDelegate = object : View.AccessibilityDelegate() {
                    override fun onInitializeAccessibilityNodeInfo(host: View, info: AccessibilityNodeInfo) {
                        super.onInitializeAccessibilityNodeInfo(host, info)
                        info.className = Button::class.java.name
                    }
                }
                setOnClickListener {
                    when (page) {
                        "hub" -> showHub()
                        "services" -> showServices()
                        "settings" -> showSettings()
                        else -> enrollment?.let { tunnel.start(it) } ?: refresh(false)
                    }
                }
            }
            item.addView(ui.icon(drawable, color), LinearLayout.LayoutParams(dp(23), dp(23)))
            item.addView(ui.text(label, 11, color, selected).apply {
                gravity = Gravity.CENTER; importantForAccessibility = View.IMPORTANT_FOR_ACCESSIBILITY_NO
                maxLines = 2; ellipsize = TextUtils.TruncateAt.END
            }, LinearLayout.LayoutParams(-1, -2).apply { topMargin = dp(3) })
            navigation.addView(item, LinearLayout.LayoutParams(0, -1, 1f).apply { marginStart = dp(3); marginEnd = dp(3) })
        }
    }

    private fun rememberHub() {
        if (currentPage != "hub") return
        if (::draft.isInitialized) draftText = draft.text.toString()
        if (::search.isInitialized) searchDraft = search.text.toString()
    }
    private fun navigate(page: String) {
        if(::phoneView.isInitialized) phoneView.clear()
        rememberHub(); generation++; scrollRevision++; forceEndQueued = false
        if(page!="hub") requestedCaptureTask=null
        currentPage = page; container.removeAllViews(); renderNavigation()
    }
    private fun showHub() {
        navigate("hub")
        val toolbar = LinearLayout(this).apply { gravity = Gravity.CENTER_VERTICAL; setPadding(0, dp(4), 0, dp(8)) }
        dayButton = ui.button(dayLabel(), R.drawable.ic_calendar, compact = true) { pickDay() }.apply {
            contentDescription = "Selecionar dia: " + dayLabel()
        }
        toolbar.addView(dayButton, LinearLayout.LayoutParams(0, dp(48), 1f).apply { marginEnd = dp(8) })
        filterButton = ui.button(feedFilters[filterIndex].first, R.drawable.ic_filter, compact = true) { pickKind() }.apply {
            contentDescription = "Filtrar atividades: " + feedFilters[filterIndex].first
        }
        toolbar.addView(filterButton, LinearLayout.LayoutParams(0, dp(48), 1f).apply { marginEnd = dp(8) })
        toolbar.addView(ui.iconButton("Pesquisar no histórico", R.drawable.ic_search) {
            searchExpanded = !searchExpanded
            searchPanel.visibility = if (searchExpanded) View.VISIBLE else View.GONE
            if (searchExpanded) {
                search.requestFocus()
                (getSystemService(INPUT_METHOD_SERVICE) as android.view.inputmethod.InputMethodManager)
                    .showSoftInput(search, android.view.inputmethod.InputMethodManager.SHOW_IMPLICIT)
            }
        }, LinearLayout.LayoutParams(dp(48), dp(48)))
        container.addView(toolbar)
        searchPanel = LinearLayout(this).apply {
            gravity = Gravity.CENTER_VERTICAL
            visibility = if (searchExpanded) View.VISIBLE else View.GONE
            setPadding(0, 0, 0, dp(8))
        }
        search = ui.edit("Pesquisar nas conversas").apply { setText(searchDraft); maxLines = 1; inputType = InputType.TYPE_CLASS_TEXT }
        searchPanel.addView(search, LinearLayout.LayoutParams(0, dp(48), 1f).apply { marginEnd = dp(8) })
        searchPanel.addView(ui.iconButton("Aplicar busca", R.drawable.ic_check) { applyFilters() },
            LinearLayout.LayoutParams(dp(48), dp(48)).apply { marginEnd = dp(8) })
        searchPanel.addView(ui.iconButton("Limpar busca", R.drawable.ic_close) { search.setText(""); applyFilters() },
            LinearLayout.LayoutParams(dp(48), dp(48)))
        container.addView(searchPanel)
        feedScroll = ScrollView(this).apply { isFillViewport = true; clipToPadding = false; setPadding(0, dp(4), 0, dp(8)) }
        feed = column()
        feedScroll.addView(feed)
        container.addView(feedScroll, LinearLayout.LayoutParams(-1, 0, 1f))
        val composer = LinearLayout(this).apply { gravity = Gravity.BOTTOM; setPadding(0, dp(8), 0, dp(4)) }
        draft = ui.edit("Mensagem para o ZaturoBot").apply {
            setText(draftText); minLines = 1; maxLines = 4
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE or InputType.TYPE_TEXT_FLAG_CAP_SENTENCES
            contentDescription = "Mensagem para o ZaturoBot"
        }
        composer.addView(draft, LinearLayout.LayoutParams(0, -2, 1f).apply { marginEnd = dp(8) })
        composer.addView(ui.iconButton("Enviar mensagem", R.drawable.ic_send, primary = true) {
            submit(draft.text.toString(), null)
        }, LinearLayout.LayoutParams(dp(52), dp(52)))
        container.addView(composer)
        renderFeed(false, forceNewest = true)
        if (visible) refresh(false)
        if(::phoneView.isInitialized) phoneView.resumePrepared()
    }

    private fun dayLabel(): String = if (selectedDay == LocalDate.now()) "Hoje"
        else selectedDay.format(DateTimeFormatter.ofPattern("dd MMM", Locale.forLanguageTag("pt-BR")))
    private fun pickDay() {
        DatePickerDialog(this, { _, year, month, day ->
            selectedDay = LocalDate.of(year, month + 1, day)
            dayButton.text = dayLabel()
            dayButton.contentDescription = "Selecionar dia: " + dayLabel()
            applyFilters(false)
        }, selectedDay.year, selectedDay.monthValue - 1, selectedDay.dayOfMonth).show()
    }
    private fun pickKind() {
        val menu = PopupMenu(this, filterButton)
        feedFilters.forEachIndexed { index, value -> menu.menu.add(0, index, index, value.first).isChecked = index == filterIndex }
        menu.menu.setGroupCheckable(0, true, true)
        menu.setOnMenuItemClickListener { item ->
            filterIndex = item.itemId
            filterButton.text = feedFilters[filterIndex].first
            filterButton.contentDescription = "Filtrar atividades: " + feedFilters[filterIndex].first
            applyFilters(false)
            true
        }
        menu.show()
    }
    private fun applyFilters(useSearchDraft: Boolean = true) {
        val candidate = if (useSearchDraft) search.text.toString() else queryText
        if (candidate.toByteArray(Charsets.UTF_8).size > 128) {
            setStatus("A busca está muito longa. Use menos palavras."); return
        }
        queryText = candidate; searchDraft = candidate
        generation++; timeline.clear(); renderFeed(false, forceNewest = true); refresh(false)
    }
    private fun maximumScroll(): Int = max(0,
        feed.height + feedScroll.paddingTop + feedScroll.paddingBottom - feedScroll.height)
    private fun atConversationEnd(): Boolean = !::feedScroll.isInitialized || !::feed.isInitialized ||
        maximumScroll() - feedScroll.scrollY <= dp(64)
    private fun scrollToEnd() { feedScroll.scrollTo(0, maximumScroll()) }
    private fun revealVisibleEnd() {
        forceEndQueued = true
        afterFeedLayout { scrollToEnd(); forceEndQueued = false }
    }
    private fun afterFeedLayout(action: () -> Unit) {
        if (!::feedScroll.isInitialized || currentPage != "hub") return
        val viewport = feedScroll
        val context = generation
        val revision = ++scrollRevision
        val listener = object : ViewTreeObserver.OnPreDrawListener {
            override fun onPreDraw(): Boolean {
                if (viewport.viewTreeObserver.isAlive) viewport.viewTreeObserver.removeOnPreDrawListener(this)
                if (currentPage == "hub" && feedScroll === viewport && generation == context && scrollRevision == revision)
                    action()
                return true
            }
        }
        viewport.viewTreeObserver.addOnPreDrawListener(listener)
        viewport.requestLayout()
        viewport.invalidate()
    }
    private fun renderFeed(append: Boolean, forceNewest: Boolean = false) {
        val oldY = feedScroll.scrollY
        val oldHeight = feed.height
        if (forceNewest) forceEndQueued = true
        val stick = forceEndQueued || atConversationEnd()
        feed.removeAllViews()
        if (timeline.items.isEmpty()) {
            val empty = column().apply { gravity = Gravity.CENTER; setPadding(dp(20), dp(40), dp(20), dp(40)) }
            empty.addView(ui.icon(R.drawable.ic_chat, MobileUi.YELLOW, 32), LinearLayout.LayoutParams(dp(48), dp(48)))
            empty.addView(ui.text(if (configuration == null) "Seu hub, sempre por perto" else "Espaço para o seu dia",
                19, bold = true).apply { gravity = Gravity.CENTER; setPadding(0, dp(16), 0, dp(8)) })
            empty.addView(ui.text(if (configuration == null) "Abra Ajustes para conectar seu aplicativo."
                else "As conversas e atualizações aparecem aqui.", 13, MobileUi.MUTED).apply { gravity = Gravity.CENTER })
            feed.addView(empty, LinearLayout.LayoutParams(-1, -2))
        } else {
            if (timeline.hasMore && timeline.items.size < 200)
                feed.addView(ui.button("Mensagens anteriores", R.drawable.ic_chat, compact = true) { refresh(true) },
                    LinearLayout.LayoutParams(-1, dp(48)).apply { bottomMargin = dp(12) })
            val date = ui.text(if (selectedDay == LocalDate.now()) "Hoje" else
                selectedDay.format(DateTimeFormatter.ofPattern("dd 'de' MMMM", Locale.forLanguageTag("pt-BR"))),
                11, MobileUi.MUTED).apply { gravity = Gravity.CENTER; setPadding(0, dp(4), 0, dp(16)) }
            feed.addView(date)
            for (item in timeline.items.asReversed()) feed.addView(messageBubble(item))
        }
        if(::phoneView.isInitialized && phoneView.hasPrepared()) feed.addView(ui.button("Abrir captura preparada",R.drawable.ic_shield){phoneView.openPrepared()})
        if(::phoneView.isInitialized) phoneView.cardView()?.let {card ->(card.parent as? android.view.ViewGroup)?.removeView(card);feed.addView(card)}
        afterFeedLayout {
            when {
                forceEndQueued -> { scrollToEnd(); forceEndQueued = false }
                append -> feedScroll.scrollTo(0, max(0, oldY + feed.height - oldHeight))
                stick -> scrollToEnd()
                else -> feedScroll.scrollTo(0, oldY)
            }
        }
    }
    private fun messageBubble(item: JSONObject): View {
        val owner = item.optString("role") in setOf("owner", "user")
        val color = if (owner) MobileUi.BG else MobileUi.WHITE
        val muted = if (owner) 0xff51440b.toInt() else MobileUi.MUTED
        val bubble = column().apply {
            setPadding(dp(14), dp(11), dp(14), dp(9))
            background = ui.rounded(if (owner) MobileUi.YELLOW else MobileUi.SURFACE, 18,
                if (owner) 0xffd4ac00.toInt() else MobileUi.STROKE).apply {
                val soft = dp(18).toFloat(); val tight = dp(6).toFloat()
                cornerRadii = if (owner) floatArrayOf(soft, soft, tight, tight, soft, soft, soft, soft)
                    else floatArrayOf(tight, tight, soft, soft, soft, soft, soft, soft)
            }
        }
        val role = if (owner) "Você" else if (item.optString("role") == "assistant") "ZaturoBot" else "Sistema"
        bubble.addView(ui.text(role, 11, if (owner) muted else MobileUi.YELLOW, true).apply {
            setPadding(0, 0, 0, dp(5))
        }, LinearLayout.LayoutParams(-2, -2))
        bubble.addView(ui.text(item.optString("text"), 15, color).apply {
            setTextIsSelectable(true); setLineSpacing(dp(2).toFloat(), 1f)
        }, LinearLayout.LayoutParams(-2, -2))
        item.optJSONObject("capture")?.let {hint ->
            if(hint.optString("source")=="android") bubble.addView(ui.button("Capturar tela atual do celular",R.drawable.ic_shield){requestMessagesCapture(hint.getString("worker"))})
            if(hint.optString("source")=="desktop") bubble.addView(ui.button("Ver tela atual do computador",R.drawable.ic_shield){phoneView.start(hint.getString("worker"),"desktop","desktop")})
        }
        if (!item.isNull("artifact_id")) bubble.addView(ui.text("Anexo disponível no computador", 11, muted).apply {
            setPadding(0, dp(8), 0, 0)
        }, LinearLayout.LayoutParams(-2, -2))
        val time = runCatching {
            Instant.ofEpochMilli(item.getLong("created_at")).atZone(ZoneId.systemDefault())
                .format(DateTimeFormatter.ofPattern("HH:mm"))
        }.getOrDefault("")
        bubble.addView(ui.text(time, 10, muted).apply { setPadding(0, dp(6), 0, 0) },
            LinearLayout.LayoutParams(-2, -2).apply { gravity = Gravity.END })
        val row = FrameLayout(this)
        row.addView(bubble, FrameLayout.LayoutParams(-2, -2, if (owner) Gravity.END else Gravity.START).apply {
            if (owner) marginStart = dp(48) else marginEnd = dp(48)
        })
        row.layoutParams = LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(10) }
        return row
    }

    private fun api(): HubApi {
        val config = configuration ?: error("Connection is not configured")
        val origin = if (enrollment != null) { require(tunnel.ready); SshTunnel.ORIGIN } else config.getString("hub_url")
        return HubApi(origin, config.getString("app_token"))
    }
    private fun refresh(append: Boolean) {
        if (!visible || currentPage != "hub" || configuration == null ||
            (enrollment != null && !tunnel.ready) || !busy.compareAndSet(false, true)) return
        val current = generation
        val day = selectedDay
        val query = queryText
        val kind = feedFilters[filterIndex].second
        val offset = if (append) timeline.nextOffset else 0
        background.execute {
            try {
                val result = api().feed(day, kind, query, offset)
                val page = result.getJSONArray("items")
                val items = (0 until page.length()).map { page.getJSONObject(it) }
                main.post {
                    if (visible && currentPage == "hub" && current == generation) {
                        val previousRows = timeline.items.map { it.toString() }
                        val previousMore = timeline.hasMore
                        timeline.merge(items, result.optBoolean("has_more"), append)
                        val revealNewest = !append && revealAfterFreshPage
                        if (revealNewest) revealAfterFreshPage = false
                        if (previousRows != timeline.items.map { it.toString() } || previousMore != timeline.hasMore || revealNewest)
                            renderFeed(append, forceNewest = revealNewest)
                        setStatus("Hub sincronizado · conexão privada", true)
                        if(android.os.SystemClock.elapsedRealtime()>requestedCaptureUntil) requestedCaptureTask=null
                        val receipt=items.firstOrNull {it.optString("task_id")==requestedCaptureTask && it.optJSONObject("capture")?.optString("source")=="desktop"}
                        if(receipt!=null && requestedCaptureTask!=null) {
                            requestedCaptureTask=null
                            val worker=receipt.getJSONObject("capture").getString("worker")
                            phoneView.start(worker,"desktop","desktop")
                        }
                    }
                }
            } catch (_: Exception) {
                main.post { if (visible && current == generation) setStatus("Hub indisponível. Toque em Reconectar.", false) }
            } finally {
                main.post {
                    busy.set(false)
                    val queued = queuedSubmission
                    queuedSubmission = null
                    if (queued != null) submit(queued.first, queued.second)
                    else if (visible && currentPage == "hub" && current != generation) refresh(false)
                }
            }
        }
    }
    private fun submit(message: String, action: String?) {
        if (message.isBlank()) return
        if(action==null) {
            val capture=PrivateCaptureCommand.parse(message)
            if(capture!=null) {
                when(capture) {
                    is PrivateCaptureCommand.Request.Page -> requestPageCapture(capture.url)
                    PrivateCaptureCommand.Request.Messages -> requestMessagesCapture()
                    PrivateCaptureCommand.Request.Choose -> requestPageCapture()
                    PrivateCaptureCommand.Request.Declined -> setStatus("Nenhuma captura foi solicitada. Use Print de página ou Print de SMS no menu Serviços quando desejar.")
                }
                return
            }
        }
        if (!busy.compareAndSet(false, true)) {
            if (!submissionRunning && queuedSubmission == null) {
                queuedSubmission = message to action
                setStatus("Preparando o envio da mensagem…")
            } else setStatus("Aguarde a confirmação do envio anterior.")
            return
        }
        submissionRunning = true
        if (action == null && currentPage == "hub") revealVisibleEnd()
        val sourcePage = currentPage
        val worker = configuration?.optString("target_worker", "work-pc")
        val previous = pending
        val request = if (previous != null && previous.message == message && previous.action == action && previous.worker == worker) previous
            else Pending(message, worker, action, UUID.randomUUID().toString()).also { pending = it }
        setStatus("Enviando solicitação…")
        background.execute {
            try {
                val reply=api().chat(request.message, request.worker, request.action, request.id)
                main.post {
                    if(request.message.lowercase().contains("print") || request.message.lowercase().contains("captura")) {requestedCaptureTask=reply.optJSONObject("task")?.optString("id");requestedCaptureUntil=android.os.SystemClock.elapsedRealtime()+360000}
                    pending = null
                    submissionRunning = false
                    if (action == null) {
                        if (currentPage == "hub" && draft.text.toString() == message) draft.setText("")
                        if (draftText == message) draftText = ""
                    }
                    val changedContext = selectedDay != LocalDate.now() || filterIndex != 0 || queryText.isNotEmpty()
                    selectedDay = LocalDate.now(); filterIndex = 0; queryText = ""; searchDraft = ""
                    if (changedContext) { generation++; timeline.clear() }
                    revealAfterFreshPage = true
                    if (currentPage == "hub") {
                        dayButton.text = dayLabel(); dayButton.contentDescription = "Selecionar dia: " + dayLabel()
                        filterButton.text = feedFilters[0].first; filterButton.contentDescription = "Filtrar atividades: Tudo"
                        search.setText(""); searchExpanded = false; searchPanel.visibility = View.GONE
                        if (changedContext) renderFeed(false, forceNewest = true) else revealVisibleEnd()
                    }
                    setStatus("Solicitação registrada no hub.", true); busy.set(false)
                    if (action != null && currentPage == sourcePage && currentPage != "hub") showHub()
                    else if (currentPage == "hub") refresh(false)
                }
            } catch (_: Exception) {
                main.post {
                    submissionRunning = false
                    setStatus("Envio não confirmado. Toque novamente para reconciliar."); busy.set(false)
                }
            }
        }
    }
    private fun requestPageCapture(initial: String="") {
        val url=ui.edit("Endereço da página (https://…)").apply {setText(initial);maxLines=3;inputType=InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI;isSaveEnabled=false}
        AlertDialog.Builder(this).setTitle("Print de página no PC").setMessage("O navegador próprio do bot abrirá este endereço e capturará a parte visível da página. Usa somente a sessão desse navegador; páginas privadas precisam de login nele. A imagem é temporária e não entra na IA nem no histórico.").setView(url)
            .setNegativeButton("Cancelar",null).setPositiveButton("Abrir e capturar") {_,_->
                val value=url.text.toString().trim()
                if(PrivateCaptureCommand.validUrl(value)) {showHub();phoneView.start(configuration?.optString("target_worker","work-pc") ?: "work-pc","browser","browser",url=value)}
                else setStatus("Informe um endereço http:// ou https:// válido, sem usuário ou senha na URL.")
            }.show()
    }
    private fun requestMessagesCapture(targetWorker: String?=null,openFirst: Boolean=false) {
        val fields=column().apply {setPadding(dp(16),0,dp(16),0)}
        val device=ui.edit("Identificador do celular USB").apply {setText("owner-phone");maxLines=1}
        val samePhone=CheckBox(this).apply {text="Estou usando o Hub nesse mesmo celular";isChecked=false;setTextColor(MobileUi.WHITE)}
        val openMessages=CheckBox(this).apply {text="Abrir Mensagens antes de capturar";isChecked=openFirst;setTextColor(MobileUi.WHITE)}
        fields.addView(device);fields.addView(openMessages);fields.addView(samePhone)
        AlertDialog.Builder(this).setTitle("Enviar print do celular?").setMessage("Envia uma imagem da tela atual, mesmo sem SMS novo ou enviado. Você pode escolher abrir Mensagens antes. Se estiver usando o Hub nesse mesmo celular, a captura será preparada para que você possa mostrar outra tela. A imagem aparece automaticamente ao voltar ao Hub e expira em até 60 segundos.").setView(fields)
            .setNegativeButton("Cancelar",null).setPositiveButton("Capturar e enviar print") {_,_->
                val id=device.text.toString()
                if(runCatching {EndpointPolicy.identifier(id)}.isSuccess) {
                    val worker=targetWorker ?: configuration?.optString("target_worker","work-pc") ?: "work-pc"
                    showHub()
                    if(openMessages.isChecked) phoneView.prepare(worker,id,true,samePhone.isChecked,autoOpen=true)
                    else if(samePhone.isChecked) phoneView.prepare(worker,id,autoOpen=true)
                    else phoneView.start(worker,id)
                }
                else setStatus("Identificador do celular inválido.")
            }.show()
    }
    private fun showServices() {
        navigate("services")
        val form = scrollForm()
        heading(form, "Serviços", "Ações do bot no computador selecionado.")
        form.addView(serviceCard("Diagnóstico", "Verifique a conexão e o ambiente do seu computador.",
            R.drawable.ic_terminal, "Executar diagnóstico") {
            submit("Execute o diagnóstico autorizado do computador selecionado.", "diagnostic")
        })
        form.addView(serviceCard("YouTube", "Contas, canais e perfis do seu serviço de vídeos.",
            R.drawable.ic_youtube, "Listar perfis") { submit("Liste os perfis do serviço YouTube.", "youtube_profiles") })
        form.addView(serviceCard("Android por USB", "Confira os dispositivos autorizados no computador.",
            R.drawable.ic_phone, "Listar dispositivos") { submit("Liste os dispositivos Android no computador.", "android_devices") })
        form.addView(serviceCard("Celular remoto", "Captura criptografada, temporária e sem salvar a imagem no histórico.",
            R.drawable.ic_shield,"Ver tela do celular") {
            val device=ui.edit("Identificador do celular registrado").apply {setText("owner-phone");maxLines=1}
            AlertDialog.Builder(this).setTitle("Visualização privada").setMessage("Ver a tela atual do celular USB no computador selecionado? A tela deve estar desbloqueada. A imagem será apagada ao sair e em até 60 segundos.").setView(device)
                .setNegativeButton("Cancelar",null).setPositiveButton("Ver tela") {_,_ ->
                    val id=device.text.toString()
                    if(runCatching {EndpointPolicy.identifier(id)}.isSuccess) {showHub();phoneView.start(configuration?.optString("target_worker","work-pc") ?: "work-pc",id)}
                    else setStatus("Identificador do celular inválido.")
                }.show()
        })
        form.addView(serviceCard("Captura deste celular", "Prepare a captura USB, abra uma tela sem dados sigilosos, aguarde 15 segundos e volte ao Hub. Expira em até 60 segundos.",R.drawable.ic_shield,"Preparar captura em 15 segundos") {
            AlertDialog.Builder(this).setTitle("Preparar uma captura USB?").setMessage("Depois de confirmar, abra uma tela desbloqueada sem dados sigilosos neste celular. O PC capturará uma única imagem após 15 segundos. Volte ao Hub e toque em Abrir captura preparada. Nenhum conteúdo entra no histórico ou na IA.")
                .setNegativeButton("Cancelar",null).setPositiveButton("Preparar") {_,_->
                    val worker=configuration?.optString("target_worker","work-pc") ?: "work-pc";showHub();phoneView.prepare(worker,"owner-phone")
                }.show()
        })
        form.addView(ui.button("Abrir captura preparada",R.drawable.ic_shield){showHub();phoneView.openPrepared()})
        form.addView(serviceCard("Print de página", "Escolha qualquer endereço público. O bot abre a página no próprio navegador e envia a imagem privada.",R.drawable.ic_shield,"Escolher página e capturar") {requestPageCapture()})
        form.addView(serviceCard("Print de SMS / tela atual", "Envia a tela atual mesmo sem SMS novo. Abrir Mensagens antes é opcional.",R.drawable.ic_phone,"Capturar e enviar print") {requestMessagesCapture()})
        form.addView(serviceCard("Mensagens do celular", "Abre Mensagens e envia um print da tela, mesmo sem SMS novo ou enviado.",R.drawable.ic_phone,"Abrir Mensagens e enviar print") {requestMessagesCapture(openFirst=true)})
        form.addView(serviceCard("Computador Linux", "Árvore de acessibilidade e navegador dedicado. Use /computador seguido da tarefa; entradas e publicações pedem confirmação.",R.drawable.ic_terminal,"Ver tela atual do computador") {
            AlertDialog.Builder(this).setTitle("Captura privada do computador?").setMessage("Capturar a janela autorizada ou aquela que você selecionar no diálogo nativo do PC? A imagem é temporária e não entra no histórico nem na IA.").setNegativeButton("Cancelar",null).setPositiveButton("Ver tela") {_,_->
                val worker=configuration?.optString("target_worker","work-pc") ?: "work-pc";showHub();phoneView.start(worker,"desktop","desktop")
            }.show()
        })
        form.addView(serviceCard("Área compartilhada", "Tarefas e instruções que o outro Codex pode consultar quando você pedir.",R.drawable.ic_folder,"Abrir área compartilhada") {showShared()})

    }
    private fun showShared() {
        navigate("shared")
        val form=scrollForm();heading(form,"Área compartilhada","Instruções são dados. O Codex executa quando você pedir, conforme as permissões locais.")
        val worker=configuration?.optString("target_worker","work-pc") ?: "work-pc"
        form.addView(ui.button("Nova tarefa",R.drawable.ic_chat) {
            val fields=column().apply {setPadding(dp(16),0,dp(16),0)}
            val title=ui.edit("Título").apply {maxLines=2};val instructions=ui.edit("Instruções para o outro computador").apply {minLines=3;maxLines=8}
            fields.addView(title,fieldParams());fields.addView(instructions,fieldParams())
            val key=UUID.randomUUID().toString()
            val dialog=AlertDialog.Builder(this).setTitle("Tarefa para "+worker).setView(fields).setNegativeButton("Cancelar",null).setPositiveButton("Salvar",null).create()
            dialog.setOnShowListener {dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val name=title.text.toString();val body=instructions.text.toString()
                if(name.isBlank() || name.toByteArray().size>160 || body.isBlank() || body.toByteArray().size>8192) {setStatus("Informe título e instruções dentro do limite.");return@setOnClickListener}
                dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled=false
                background.execute {try {api().sharedCreate(worker,name,body,key);main.post {dialog.dismiss();if(visible && currentPage=="shared") showShared()}} catch (_: Exception) {main.post {dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled=true;setStatus("Salvamento não confirmado. Tente novamente sem alterar o texto para reconciliar.")}}}
            }};dialog.show()
        },fieldParams())
        form.addView(ui.button("Atualizar tarefas",R.drawable.ic_sync){showShared()},fieldParams())
        val current=generation
        if(configuration==null) {form.addView(ui.text("Configure a conexão nos Ajustes.",14,MobileUi.MUTED));return}
        background.execute {try {
            val items=api().sharedList(worker)
            main.post {if(!visible || currentPage!="shared" || current!=generation) return@post
                if(items.length()==0) form.addView(ui.text("Nenhuma tarefa compartilhada para este computador.",14,MobileUi.MUTED))
                for(index in 0 until items.length()) {
                    val item=items.getJSONObject(index);val card=panel()
                    card.addView(ui.text(item.getString("title"),18,bold=true));card.addView(ui.text(sharedState(item.getString("state"))+" · revisão "+item.getInt("revision"),12,MobileUi.MUTED))
                    card.addView(ui.button("Ler instruções",R.drawable.ic_folder) {
                        background.execute {try {val detail=api().sharedRead(item.getString("id"));main.post {if(visible && currentPage=="shared") showSharedDetail(detail)}} catch (_: Exception) {main.post {setStatus("Não foi possível ler esta tarefa.")}}}
                    });form.addView(card)
                }
            }
        } catch (_: Exception) {main.post {if(visible && currentPage=="shared") setStatus("Área compartilhada indisponível. Verifique a conexão e a versão do hub.")}}}
    }
    private fun sharedState(value: String): String = when(value) {"pending"->"Pendente";"claimed"->"Em execução";"blocked"->"Aguardando ação";"done"->"Concluída";else->"Atualizada"}
    private fun showSharedDetail(detail: JSONObject) {
        val task=detail.getJSONObject("task");val notes=detail.getJSONArray("notes")
        val text=task.getString("instructions")+"\n\n"+sharedState(task.getString("state"))+"\n\n"+(0 until notes.length()).reversed().joinToString("\n\n") {n ->val note=notes.getJSONObject(n);note.getString("author")+": "+note.getString("note")}
        val builder=AlertDialog.Builder(this).setTitle(task.getString("title")).setMessage(text).setPositiveButton("Fechar",null)
        if(task.getString("state")!="done") builder.setNeutralButton("Responder") {_,_ ->
            val field=ui.edit("Escreva uma atualização para o outro computador").apply {minLines=3;maxLines=8}
            val key=UUID.randomUUID().toString()
            val dialog=AlertDialog.Builder(this).setTitle("Resposta compartilhada").setView(field).setNegativeButton("Cancelar",null).setPositiveButton("Enviar",null).create()
            dialog.setOnShowListener {dialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener {
                val note=field.text.toString()
                if(note.isBlank() || note.toByteArray().size>4096) {setStatus("Escreva uma resposta dentro do limite.");return@setOnClickListener}
                dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled=false
                background.execute {try {api().sharedUpdate(task.getString("id"),task.getInt("revision"),task.getString("state"),note,key);main.post {dialog.dismiss();if(visible && currentPage=="shared") showShared()}} catch (_: Exception) {main.post {dialog.getButton(AlertDialog.BUTTON_POSITIVE).isEnabled=true;setStatus("Resposta não confirmada ou tarefa atualizada por outro agente. Reabra a tarefa para conferir.")}}}
            }};dialog.show()
        }
        builder.show()
    }
    private fun serviceCard(name: String, description: String, icon: Int, label: String, action: () -> Unit): View {
        val card = panel()
        val top = LinearLayout(this).apply { gravity = Gravity.CENTER_VERTICAL }
        top.addView(ui.icon(icon, MobileUi.YELLOW), LinearLayout.LayoutParams(dp(28), dp(28)).apply { marginEnd = dp(12) })
        top.addView(ui.text(name, 19, bold = true))
        card.addView(top)
        card.addView(ui.text(description, 13, MobileUi.MUTED).apply {
            setLineSpacing(dp(3).toFloat(), 1f); setPadding(0, dp(10), 0, dp(16))
        })
        card.addView(ui.button(label, icon, action = action), LinearLayout.LayoutParams(-1, -2))
        return card
    }
    private fun showSettings() {
        navigate("settings")
        val form = scrollForm()
        heading(form, "Ajustes", "Seu hub e o computador de execução.")
        val connection = panel()
        connection.addView(ui.text("Conexão privada", 18, bold = true))
        connection.addView(ui.text(if (enrollment != null) "SSH configurado com chave exclusiva do aplicativo."
            else "Importe o arquivo privado emitido pelo seu hub.", 13, MobileUi.MUTED).apply {
            setPadding(0, dp(10), 0, dp(16))
        })
        connection.addView(ui.button("Importar conexão privada", R.drawable.ic_shield, primary = configuration == null) {
            startActivityForResult(Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
                addCategory(Intent.CATEGORY_OPENABLE); type = "application/json"
            }, IMPORT_CONNECTION)
        }, LinearLayout.LayoutParams(-1, -2))
        form.addView(connection)
        val manual = panel()
        manual.addView(ui.text("Conexão manual", 18, bold = true))
        manual.addView(ui.text("Para uma origem HTTPS ou um túnel local existente.", 13, MobileUi.MUTED).apply {
            setPadding(0, dp(10), 0, dp(16))
        })
        val origin = ui.edit("Endereço do hub").apply {
            setText(configuration?.optString("hub_url", "") ?: "")
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_URI; maxLines = 1
        }
        manual.addView(origin, fieldParams())
        val token = ui.edit("Token do aplicativo").apply {
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
            importantForAutofill = View.IMPORTANT_FOR_AUTOFILL_NO
        }
        manual.addView(token, fieldParams())
        manual.addView(ui.text("Deixe vazio para manter o token atual.", 11, MobileUi.MUTED).apply {
            setPadding(dp(2), 0, dp(2), dp(12))
        })
        val worker = ui.edit("Computador de execução").apply {
            setText(configuration?.optString("target_worker", "work-pc") ?: "work-pc"); maxLines = 1
        }
        manual.addView(worker, fieldParams())
        manual.addView(ui.button("Salvar conexão", R.drawable.ic_check) {
            try {
                val value = JSONObject().put("hub_url", EndpointPolicy.validate(origin.text.toString()))
                    .put("app_token", EndpointPolicy.token(token.text.toString().ifEmpty { configuration?.optString("app_token", "") ?: "" }))
                    .put("target_worker", EndpointPolicy.identifier(worker.text.toString()))
                privateSettings.save(value); configuration = value; enrollment = null; tunnel.stop(); token.setText("")
                timeline.clear(); showHub()
            } catch (_: Exception) { setStatus("Confira a origem HTTPS ou local e a credencial.") }
        }, LinearLayout.LayoutParams(-1, -2))
        form.addView(manual)
        form.addView(ui.text("Credenciais protegidas pelo Android Keystore. A conexão permanece ativa enquanto o aplicativo está aberto.",
            12, MobileUi.MUTED).apply { setPadding(dp(4), dp(4), dp(4), dp(20)); setLineSpacing(dp(3).toFloat(), 1f) })
        form.addView(ui.button("Remover conexão deste aplicativo", R.drawable.ic_close, danger = true) {
            privateSettings.clear(); configuration = null; enrollment = null; tunnel.stop(); timeline.clear()
            setStatus("Conexão local removida.", false); showSettings()
        }, LinearLayout.LayoutParams(-1, -2))
    }

    @Deprecated("Activity result override for native platform Activity")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != IMPORT_CONNECTION || resultCode != RESULT_OK) return
        try {
            val uri = data?.data ?: return
            val bytes = contentResolver.openInputStream(uri)!!.use { input ->
                val out = java.io.ByteArrayOutputStream(); val buffer = ByteArray(4096)
                while (true) { val count = input.read(buffer); if (count < 0) break; require(out.size() + count <= 32768); out.write(buffer, 0, count) }
                out.toByteArray()
            }
            val value = JSONObject(String(bytes, Charsets.UTF_8)); bytes.fill(0)
            require(value.getInt("schema") == 1)
            val imported = parseEnrollmentOrNull(value) ?: error("Invalid enrollment")
            privateSettings.save(value); configuration = value; enrollment = imported; timeline.clear()
            setStatus("Conexão importada com segurança."); showHub(); if (visible) tunnel.start(imported)
        } catch (_: Exception) { setStatus("Arquivo de conexão inválido. Sua configuração foi preservada.") }
    }
    private fun parseEnrollmentOrNull(value: JSONObject): Enrollment? = try {
        if (value.optInt("schema") != 1) null else {
            val decoded = Base64.getDecoder().decode(value.getString("ssh_private_key_b64")); require(decoded.size <= 16384)
            val key = String(decoded, Charsets.UTF_8); decoded.fill(0)
            Enrollment(value.getString("client_id"), value.getString("ssh_host"), value.getInt("ssh_port"), value.getString("ssh_user"), key,
                value.getString("host_key_sha256"), value.getString("app_token"), value.getString("target_worker"))
        }
    } catch (_: Exception) { null }
    private fun setStatus(message: String, connected: Boolean? = null) {
        if (status.text.toString() != message) status.text = message
        connected?.let {
            val color = if (it) 0xff76c799.toInt() else MobileUi.MUTED
            statusDot.background = ui.rounded(color, 8, color)
        }
    }
    private fun column() = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
    private fun scrollForm(): LinearLayout {
        val scroll = ScrollView(this).apply { clipToPadding = false }
        val form = column().apply { setPadding(0, dp(12), 0, dp(16)) }
        scroll.addView(form); container.addView(scroll, LinearLayout.LayoutParams(-1, -1)); return form
    }
    private fun heading(form: LinearLayout, title: String, subtitle: String) {
        form.addView(ui.text(title, 25, bold = true).apply { if (Build.VERSION.SDK_INT >= 28) isAccessibilityHeading = true })
        form.addView(ui.text(subtitle, 13, MobileUi.MUTED).apply { setPadding(0, dp(8), 0, dp(22)) })
    }
    private fun panel() = column().apply {
        background = ui.rounded(); setPadding(dp(18), dp(18), dp(18), dp(18))
        layoutParams = LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(14) }
    }
    private fun fieldParams() = LinearLayout.LayoutParams(-1, -2).apply { bottomMargin = dp(12) }
    private fun dp(value: Int) = ui.dp(value)
    companion object { private const val IMPORT_CONNECTION = 41 }
}
