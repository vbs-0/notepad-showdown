package com.vbs.knotepad

import android.app.Activity
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.text.Editable
import android.text.InputType
import android.text.TextWatcher
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.inputmethod.InputMethodManager
import android.widget.EditText
import android.widget.FrameLayout
import org.json.JSONArray
import org.json.JSONObject
import java.io.File
import kotlin.math.abs
import kotlin.math.max

class Note(var id: Int, var updated: Long, var title: String, var body: String)

class MainActivity : Activity() {

    // ---- state -----------------------------------------------------------
    private val notes = mutableListOf<Note>()
    private var screen = 0                 // 0 list, 1 editor
    private var currentId: Int? = null
    private var query = ""
    private var listScroll = 0f
    private var contentH = 0f
    private var status = ""
    private var dirty = false
    private var syncing = false

    private lateinit var board: Board
    private lateinit var searchEt: EditText
    private lateinit var titleEt: EditText
    private lateinit var bodyEt: EditText

    private var d = 1f
    private var wPx = 0f
    private var hPx = 0f

    private val ui = Handler(Looper.getMainLooper())
    private val saveTick = object : Runnable {
        override fun run() {
            if (dirty) {
                persist()
                dirty = false
                status = wordStatus()
                board.invalidate()
            }
        }
    }

    companion object {
        const val BG = 0xFFF4F2ED.toInt()
        const val SURFACE = 0xFFFFFFFF.toInt()
        const val INK = 0xFF171614.toInt()
        const val INK2 = 0xFF6B675E.toInt()
        const val INK3 = 0xFF9C978C.toInt()
        const val LINE = 0xFFE6E1D6.toInt()
        const val ACCENT = 0xFFC0562F.toInt()
        const val DANGER = 0xFFA83A2C.toInt()
        const val LIST_Y = 166f
        const val ITEM_H = 108f
        const val CARD_H = 96f
    }

    // ---- lifecycle -------------------------------------------------------
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        d = resources.displayMetrics.density
        load()

        val root = FrameLayout(this)
        root.setBackgroundColor(BG)

        board = Board()
        root.addView(board, FrameLayout.LayoutParams(-1, -1))

        searchEt = input("Search notes", 14f, false, singleLine = true, centerV = true)
        titleEt = input("Title", 26f, true, singleLine = true, centerV = true)
        bodyEt = input("Start writing…", 16f, false, singleLine = false, centerV = false)

        root.addView(searchEt)
        root.addView(titleEt)
        root.addView(bodyEt)

        val watcher = object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
            override fun onTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
            override fun afterTextChanged(s: Editable?) {
                if (syncing) return
                val id = currentId ?: return
                val n = notes.firstOrNull { it.id == id } ?: return
                n.title = titleEt.text.toString()
                n.body = bodyEt.text.toString()
                n.updated = System.currentTimeMillis() / 1000
                dirty = true
                status = wordStatus()
                ui.removeCallbacks(saveTick)
                ui.postDelayed(saveTick, 800)
                board.invalidate()
            }
        }
        titleEt.addTextChangedListener(watcher)
        bodyEt.addTextChangedListener(watcher)

        searchEt.addTextChangedListener(object : TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
            override fun onTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
            override fun afterTextChanged(s: Editable?) {
                query = s?.toString() ?: ""
                board.invalidate()
            }
        })

        setContentView(root)
        board.isFocusableInTouchMode = true
        board.requestFocus()
        showScreen()
        status = wordStatus()
    }

    override fun onPause() {
        super.onPause()
        if (dirty) {
            persist()
            dirty = false
        }
    }

    // ---- view construction ----------------------------------------------
    private fun input(
        hint: String,
        size: Float,
        bold: Boolean,
        singleLine: Boolean,
        centerV: Boolean
    ): EditText {
        val e = EditText(this)
        e.background = null
        e.setPadding(0, 0, 0, 0)
        e.includeFontPadding = false
        e.hint = hint
        e.textSize = size
        e.setTextColor(INK)
        e.setHintTextColor(INK3)
        e.typeface = if (bold) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
        e.inputType = if (singleLine)
            InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
        else
            InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
        e.isSingleLine = singleLine
        e.gravity = (if (centerV) Gravity.CENTER_VERTICAL else Gravity.TOP) or Gravity.START
        return e
    }

    private fun layoutInputs() {
        val wd = wPx / d
        val hd = hPx / d
        fun lp(x: Float, y: Float, ww: Float, hh: Float): FrameLayout.LayoutParams {
            val p = FrameLayout.LayoutParams((ww * d).toInt(), (hh * d).toInt())
            p.leftMargin = (x * d).toInt()
            p.topMargin = (y * d).toInt()
            return p
        }
        searchEt.layoutParams = lp(22f, 108f, wd - 44f, 46f)
        titleEt.layoutParams = lp(24f, 80f, wd - 48f, 40f)
        bodyEt.layoutParams = lp(24f, 152f, wd - 48f, max(60f, hd - 200f))
    }

    private fun showScreen() {
        searchEt.visibility = if (screen == 0) View.VISIBLE else View.GONE
        titleEt.visibility = if (screen == 1) View.VISIBLE else View.GONE
        bodyEt.visibility = if (screen == 1) View.VISIBLE else View.GONE
        if (screen == 1) {
            val n = notes.firstOrNull { it.id == currentId }
            syncing = true
            titleEt.setText(n?.title ?: "")
            bodyEt.setText(n?.body ?: "")
            syncing = false
        }
        board.invalidate()
    }

    // ---- model -----------------------------------------------------------
    private fun file() = File(filesDir, "notes.json")

    private fun load() {
        val f = file()
        if (!f.exists()) return
        try {
            val arr = JSONArray(f.readText())
            for (i in 0 until arr.length()) {
                val o = arr.getJSONObject(i)
                notes.add(
                    Note(
                        o.optInt("id"),
                        o.optLong("updated"),
                        o.optString("title"),
                        o.optString("body")
                    )
                )
            }
        } catch (_: Exception) {
        }
        notes.sortByDescending { it.updated }
    }

    private fun persist() {
        try {
            val arr = JSONArray()
            for (n in notes) {
                arr.put(
                    JSONObject()
                        .put("id", n.id)
                        .put("updated", n.updated)
                        .put("title", n.title)
                        .put("body", n.body)
                )
            }
            file().writeText(arr.toString())
        } catch (_: Exception) {
        }
    }

    private fun wordStatus(): String {
        val id = currentId ?: return ""
        val n = notes.firstOrNull { it.id == id } ?: return ""
        return "${n.body.trim().split(Regex("\\s+")).count { it.isNotEmpty() }} words · Saved"
    }

    private fun newNote() {
        val id = (notes.maxOfOrNull { it.id } ?: 0) + 1
        notes.add(Note(id, System.currentTimeMillis() / 1000, "", ""))
        notes.sortByDescending { it.updated }
        currentId = id
        screen = 1
        status = "0 words · Saved"
        persist()
        showScreen()
        titleEt.requestFocus()
        getSystemService(INPUT_METHOD_SERVICE)?.let {
            (it as InputMethodManager).showSoftInput(titleEt, InputMethodManager.SHOW_IMPLICIT)
        }
    }

    private fun open(id: Int) {
        currentId = id
        screen = 1
        status = wordStatus()
        showScreen()
    }

    private fun back() {
        if (dirty) {
            persist()
            dirty = false
        }
        notes.sortByDescending { it.updated }
        currentId = null
        screen = 0
        status = ""
        getSystemService(INPUT_METHOD_SERVICE)?.let {
            (it as InputMethodManager).hideSoftInputFromWindow(bodyEt.windowToken, 0)
        }
        showScreen()
    }

    private fun delete() {
        val id = currentId ?: return
        notes.removeAll { it.id == id }
        currentId = null
        screen = 0
        dirty = false
        persist()
        showScreen()
    }

    private fun filtered(): List<Note> =
        if (query.isBlank()) notes
        else notes.filter {
            it.title.contains(query, true) || it.body.contains(query, true)
        }

    // ---- drawing ---------------------------------------------------------
    private val p = Paint(Paint.ANTI_ALIAS_FLAG)
    private val rect = RectF()

    private fun txt(c: Canvas, s: String, x: Float, y: Float, size: Float, color: Int, bold: Boolean) {
        p.style = Paint.Style.FILL
        p.color = color
        p.textSize = size * d
        p.typeface = if (bold) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
        c.drawText(s, x * d, y * d, p)
    }

    private fun centerTxt(c: Canvas, s: String, cx: Float, y: Float, size: Float, color: Int, bold: Boolean) {
        p.style = Paint.Style.FILL
        p.textSize = size * d
        p.typeface = if (bold) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
        val tw = p.measureText(s) / d
        txt(c, s, cx - tw / 2f, y, size, color, bold)
    }

    private fun roundRect(c: Canvas, x: Float, y: Float, ww: Float, hh: Float, r: Float, color: Int) {
        p.style = Paint.Style.FILL
        p.color = color
        rect.set(x * d, y * d, (x + ww) * d, (y + hh) * d)
        c.drawRoundRect(rect, r * d, r * d, p)
    }

    private fun circle(c: Canvas, cx: Float, cy: Float, r: Float, color: Int) {
        p.style = Paint.Style.FILL
        p.color = color
        c.drawCircle(cx * d, cy * d, r * d, p)
    }

    private fun line(c: Canvas, x1: Float, y1: Float, x2: Float, y2: Float, width: Float, color: Int) {
        p.style = Paint.Style.STROKE
        p.strokeWidth = width * d
        p.strokeCap = Paint.Cap.ROUND
        p.color = color
        c.drawLine(x1 * d, y1 * d, x2 * d, y2 * d, p)
        p.style = Paint.Style.FILL
    }

    private fun elide(s: String, size: Float, bold: Boolean, maxW: Float): String {
        p.textSize = size * d
        p.typeface = if (bold) Typeface.DEFAULT_BOLD else Typeface.DEFAULT
        if (p.measureText(s) <= maxW * d) return s
        var cut = s
        while (cut.isNotEmpty() && p.measureText("$cut…") > maxW * d) cut = cut.dropLast(1)
        return "$cut…"
    }

    private fun drawList(c: Canvas) {
        val wd = wPx / d
        txt(c, "Notes", 24f, 62f, 34f, INK, true)
        val n = notes.size
        txt(c, if (n == 1) "1 note" else "$n notes", 24f, 88f, 13f, INK3, false)

        circle(c, wd - 50f, 54f, 26f, ACCENT)
        centerTxt(c, "+", wd - 50f, 66f, 30f, SURFACE, false)

        roundRect(c, 22f, 108f, wd - 44f, 46f, 14f, SURFACE)

        val list = filtered()
        contentH = list.size * ITEM_H
        val viewH = hPx / d - LIST_Y
        c.save()
        c.clipRect(0f, LIST_Y * d, wPx, (LIST_Y + viewH) * d)

        if (list.isEmpty()) {
            val cx = wd / 2f
            val cy = LIST_Y + viewH / 2f
            roundRect(c, cx - 39f, cy - 90f, 78f, 78f, 26f, 0xFFF6E7DF.toInt())
            rect.set((cx - 16f) * d, (cy - 63f) * d, (cx + 16f) * d, (cy - 61f) * d)
            p.color = ACCENT; p.style = Paint.Style.FILL; c.drawRect(rect, p)
            rect.set((cx - 16f) * d, (cy - 53f) * d, (cx + 6f) * d, (cy - 51f) * d)
            c.drawRect(rect, p)
            rect.set((cx - 16f) * d, (cy - 43f) * d, (cx + 11f) * d, (cy - 41f) * d)
            c.drawRect(rect, p)
            centerTxt(c, if (query.isBlank()) "No notes yet" else "Nothing found", cx, cy - 2f, 17f, INK, true)
            centerTxt(
                c,
                if (query.isBlank()) "Tap + to write your first one." else "Try a different word.",
                cx, cy + 22f, 13f, INK3, false
            )
        } else {
            for ((i, note) in list.withIndex()) {
                val y = LIST_Y + i * ITEM_H - listScroll
                if (y + CARD_H < LIST_Y || y > LIST_Y + viewH) continue
                roundRect(c, 22f, y, wd - 44f, CARD_H, 16f, SURFACE)
                val title = if (note.title.isBlank()) "Untitled" else note.title
                txt(c, elide(title, 16f, true, wd - 90f), 40f, y + 32f, 16f, INK, true)
                txt(c, elide(preview(note), 13f, false, wd - 90f), 40f, y + 54f, 13f, INK2, false)
                txt(c, relTime(note.updated), 40f, y + 75f, 11f, INK3, false)
                line(c, wd - 52f, y + 42f, wd - 46f, y + 48f, 1.6f, INK3)
                line(c, wd - 46f, y + 48f, wd - 52f, y + 54f, 1.6f, INK3)
            }
        }
        c.restore()
    }

    private fun drawEditor(c: Canvas) {
        val wd = wPx / d
        line(c, 24f, 22f, 17f, 30f, 2f, INK)
        line(c, 17f, 30f, 24f, 38f, 2f, INK)

        roundRect(c, wd - 104f, 17f, 88f, 34f, 17f, SURFACE)
        centerTxt(c, "Delete", wd - 60f, 39f, 13f, DANGER, true)

        rect.set(24f * d, 136f * d, (wd - 24f) * d, 137f * d)
        p.color = LINE; p.style = Paint.Style.FILL; c.drawRect(rect, p)

        txt(c, status, 24f, hPx / d - 16f, 11.5f, INK3, false)
    }

    private fun preview(n: Note): String {
        val b = n.body.trim()
        if (b.isEmpty()) return "No additional text"
        return b.lineSequence().firstOrNull()?.take(90) ?: ""
    }

    private fun relTime(updated: Long): String {
        val age = System.currentTimeMillis() / 1000 - updated
        return when {
            age < 60 -> "just now"
            age < 3600 -> "${age / 60}m ago"
            age < 86400 -> "${age / 3600}h ago"
            age < 604800 -> "${age / 86400}d ago"
            age < 2592000 -> "${age / 604800}w ago"
            else -> "${age / 2592000}mo ago"
        }
    }

    // ---- touch -----------------------------------------------------------
    private var downX = 0f
    private var downY = 0f
    private var downOk = false
    private var moved = false

    private fun tap(x: Float, y: Float) {
        if (screen == 0) {
            val wd = wPx / d
            val dx = x - (wd - 50f)
            val dy = y - 54f
            if (dx * dx + dy * dy <= 26f * 26f) {
                newNote()
                return
            }
            if (y >= LIST_Y) {
                val idx = ((y - LIST_Y + listScroll) / ITEM_H).toInt()
                val list = filtered()
                if (idx in list.indices) {
                    open(list[idx].id)
                }
            }
        } else {
            if (x <= 60f && y <= 60f) {
                back()
                return
            }
            if (x >= wPx / d - 104f && y <= 55f) {
                delete()
            }
        }
    }

    inner class Board : View(this@MainActivity) {
        override fun onSizeChanged(ww: Int, hh: Int, ow: Int, oh: Int) {
            wPx = ww.toFloat()
            hPx = hh.toFloat()
            layoutInputs()
        }

        override fun onDraw(canvas: Canvas) {
            canvas.drawColor(BG)
            if (screen == 0) drawList(canvas) else drawEditor(canvas)
        }

        override fun onTouchEvent(e: MotionEvent): Boolean {
            val x = e.x / d
            val y = e.y / d
            when (e.actionMasked) {
                MotionEvent.ACTION_DOWN -> {
                    downX = x; downY = y; downOk = true; moved = false
                }
                MotionEvent.ACTION_MOVE -> {
                    if (downOk) {
                        val dy = y - downY
                        if (abs(dy) > 6f) {
                            moved = true
                            if (screen == 0) {
                                val viewH = hPx / d - LIST_Y
                                val maxScroll = max(0f, contentH - viewH)
                                listScroll = (listScroll - dy).coerceIn(0f, maxScroll)
                                invalidate()
                            }
                            downY = y
                        }
                    }
                }
                MotionEvent.ACTION_UP -> {
                    if (downOk && !moved && abs(x - downX) < 12f && abs(y - downY) < 12f) {
                        tap(x, y)
                    }
                    downOk = false
                }
                MotionEvent.ACTION_CANCEL -> downOk = false
            }
            return true
        }
    }
}
