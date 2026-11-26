import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path_provider/path_provider.dart';

void main() => runApp(const NotepadApp());

// ---- palette (same values as the Rust and Kotlin builds) -------------------
const kBg = Color(0xFFF4F2ED);
const kSurface = Color(0xFFFFFFFF);
const kInk = Color(0xFF171614);
const kInk2 = Color(0xFF6B675E);
const kInk3 = Color(0xFF9C978C);
const kLine = Color(0xFFE6E1D6);
const kAccent = Color(0xFFC0562F);
const kAccentSoft = Color(0xFFF6E7DF);
const kDanger = Color(0xFFA83A2C);

class Note {
  int id;
  String title;
  String body;
  int updated;

  Note({required this.id, this.title = '', this.body = '', required this.updated});

  Map<String, dynamic> toJson() =>
      {'id': id, 'title': title, 'body': body, 'updated': updated};

  factory Note.fromJson(Map<String, dynamic> j) => Note(
        id: (j['id'] as num?)?.toInt() ?? 0,
        title: j['title'] as String? ?? '',
        body: j['body'] as String? ?? '',
        updated: (j['updated'] as num?)?.toInt() ?? 0,
      );
}

class NotepadApp extends StatelessWidget {
  const NotepadApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Notepad D',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        useMaterial3: false,
        scaffoldBackgroundColor: kBg,
        fontFamily: 'Roboto',
      ),
      home: const HomePage(),
    );
  }
}

class HomePage extends StatefulWidget {
  const HomePage({super.key});

  @override
  State<HomePage> createState() => _HomePageState();
}

class _HomePageState extends State<HomePage> {
  final List<Note> _notes = [];
  int _screen = 0; // 0 list, 1 editor
  int? _currentId;
  String _query = '';
  String _status = '';
  bool _loaded = false;

  final _searchCtrl = TextEditingController();
  final _titleCtrl = TextEditingController();
  final _bodyCtrl = TextEditingController();
  final _titleFocus = FocusNode();
  final _bodyFocus = FocusNode();
  Timer? _saveTimer;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _saveTimer?.cancel();
    _searchCtrl.dispose();
    _titleCtrl.dispose();
    _bodyCtrl.dispose();
    _titleFocus.dispose();
    _bodyFocus.dispose();
    super.dispose();
  }

  // ---- storage -----------------------------------------------------------
  Future<File> _file() async {
    final dir = await getApplicationDocumentsDirectory();
    return File('${dir.path}/notes.json');
  }

  Future<void> _load() async {
    try {
      final f = await _file();
      if (await f.exists()) {
        final raw = jsonDecode(await f.readAsString()) as List;
        _notes
          ..clear()
          ..addAll(raw.map((e) => Note.fromJson(e as Map<String, dynamic>)));
      }
    } catch (_) {}
    _notes.sort((a, b) => b.updated.compareTo(a.updated));
    if (mounted) setState(() => _loaded = true);
  }

  Future<void> _persist() async {
    try {
      final f = await _file();
      await f.writeAsString(jsonEncode(_notes.map((n) => n.toJson()).toList()));
    } catch (_) {}
  }

  void _touch() {
    final id = _currentId;
    if (id == null) return;
    final n = _notes.firstWhere((e) => e.id == id);
    n.title = _titleCtrl.text;
    n.body = _bodyCtrl.text;
    n.updated = DateTime.now().millisecondsSinceEpoch ~/ 1000;
    setState(() => _status = _wordStatus(n.body, saved: false));
    _saveTimer?.cancel();
    _saveTimer = Timer(const Duration(milliseconds: 800), () async {
      await _persist();
      if (mounted) setState(() => _status = _wordStatus(_bodyCtrl.text));
    });
  }

  String _wordStatus(String body, {bool saved = true}) {
    final words = body.trim().isEmpty
        ? 0
        : body.trim().split(RegExp(r'\s+')).where((w) => w.isNotEmpty).length;
    return '$words words · ${saved ? 'Saved' : 'Saving…'}';
  }

  // ---- actions -----------------------------------------------------------
  void _newNote() {
    final id = _notes.isEmpty
        ? 1
        : _notes.map((e) => e.id).reduce((a, b) => a > b ? a : b) + 1;
    final n = Note(id: id, updated: DateTime.now().millisecondsSinceEpoch ~/ 1000);
    setState(() {
      _notes.add(n);
      _notes.sort((a, b) => b.updated.compareTo(a.updated));
      _currentId = id;
      _screen = 1;
      _status = _wordStatus('');
    });
    _titleCtrl.text = '';
    _bodyCtrl.text = '';
    _persist();
    _titleFocus.requestFocus();
  }

  void _open(Note n) {
    setState(() {
      _currentId = n.id;
      _screen = 1;
      _status = _wordStatus(n.body);
    });
    _titleCtrl.text = n.title;
    _bodyCtrl.text = n.body;
  }

  Future<void> _back() async {
    _saveTimer?.cancel();
    await _persist();
    FocusScope.of(context).unfocus();
    setState(() {
      _notes.sort((a, b) => b.updated.compareTo(a.updated));
      _currentId = null;
      _screen = 0;
      _status = '';
    });
  }

  Future<void> _delete() async {
    final id = _currentId;
    if (id == null) return;
    FocusScope.of(context).unfocus();
    setState(() {
      _notes.removeWhere((e) => e.id == id);
      _currentId = null;
      _screen = 0;
    });
    await _persist();
  }

  List<Note> get _filtered {
    final q = _query.trim().toLowerCase();
    if (q.isEmpty) return _notes;
    return _notes
        .where((n) =>
            n.title.toLowerCase().contains(q) ||
            n.body.toLowerCase().contains(q))
        .toList();
  }

  String _relTime(int updated) {
    final age = DateTime.now().millisecondsSinceEpoch ~/ 1000 - updated;
    if (age < 60) return 'just now';
    if (age < 3600) return '${age ~/ 60}m ago';
    if (age < 86400) return '${age ~/ 3600}h ago';
    if (age < 604800) return '${age ~/ 86400}d ago';
    if (age < 2592000) return '${age ~/ 604800}w ago';
    return '${age ~/ 2592000}mo ago';
  }

  // ---- build -------------------------------------------------------------
  @override
  Widget build(BuildContext context) {
    if (!_loaded) {
      return const Scaffold(backgroundColor: kBg);
    }
    return Scaffold(
      backgroundColor: kBg,
      resizeToAvoidBottomInset: true,
      body: SafeArea(
        child: _screen == 0 ? _buildList() : _buildEditor(),
      ),
    );
  }

  Widget _buildList() {
    final list = _filtered;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(24, 26, 24, 0),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    const Text(
                      'Notes',
                      style: TextStyle(
                        fontSize: 34,
                        fontWeight: FontWeight.w700,
                        color: kInk,
                        letterSpacing: -1,
                      ),
                    ),
                    const SizedBox(height: 6),
                    Text(
                      _notes.length == 1 ? '1 note' : '${_notes.length} notes',
                      style: const TextStyle(fontSize: 13, color: kInk3),
                    ),
                  ],
                ),
              ),
              _plusButton(),
            ],
          ),
        ),
        const SizedBox(height: 22),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 22),
          child: Container(
            height: 46,
            padding: const EdgeInsets.symmetric(horizontal: 16),
            decoration: BoxDecoration(
              color: kSurface,
              borderRadius: BorderRadius.circular(14),
              border: Border.all(color: kLine),
            ),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _searchCtrl,
                    style: const TextStyle(fontSize: 14, color: kInk),
                    cursorColor: kInk,
                    decoration: const InputDecoration(
                      isDense: true,
                      border: InputBorder.none,
                      hintText: 'Search notes',
                      hintStyle: TextStyle(fontSize: 14, color: kInk3),
                    ),
                    onChanged: (v) => setState(() => _query = v),
                  ),
                ),
                if (_query.isNotEmpty)
                  GestureDetector(
                    onTap: () {
                      _searchCtrl.clear();
                      setState(() => _query = '');
                    },
                    child: const Icon(Icons.close, size: 18, color: kInk3),
                  ),
              ],
            ),
          ),
        ),
        const SizedBox(height: 12),
        Expanded(
          child: list.isEmpty
              ? _emptyState()
              : ListView.builder(
                  padding: const EdgeInsets.fromLTRB(22, 6, 22, 24),
                  itemCount: list.length,
                  itemBuilder: (_, i) => _card(list[i]),
                ),
        ),
      ],
    );
  }

  Widget _plusButton() {
    return GestureDetector(
      onTap: _newNote,
      child: Container(
        width: 52,
        height: 52,
        decoration: BoxDecoration(
          color: kAccent,
          shape: BoxShape.circle,
          boxShadow: [
            BoxShadow(
              color: kAccent.withOpacity(0.28),
              blurRadius: 20,
              offset: const Offset(0, 8),
            ),
          ],
        ),
        child: const Center(
          child: Text(
            '+',
            style: TextStyle(fontSize: 30, color: Colors.white, height: 1),
          ),
        ),
      ),
    );
  }

  Widget _emptyState() {
    final searching = _query.isNotEmpty;
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 78,
            height: 78,
            decoration: BoxDecoration(
              color: kAccentSoft,
              borderRadius: BorderRadius.circular(26),
            ),
            child: Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  _bar(32),
                  const SizedBox(height: 8),
                  _bar(22),
                  const SizedBox(height: 8),
                  _bar(27),
                ],
              ),
            ),
          ),
          const SizedBox(height: 14),
          Text(
            searching ? 'Nothing found' : 'No notes yet',
            style: const TextStyle(
              fontSize: 17,
              fontWeight: FontWeight.w600,
              color: kInk,
            ),
          ),
          const SizedBox(height: 6),
          Text(
            searching ? 'Try a different word.' : 'Tap + to write your first one.',
            style: const TextStyle(fontSize: 13, color: kInk3),
          ),
        ],
      ),
    );
  }

  Widget _bar(double w) => Container(
        width: w,
        height: 2,
        decoration: BoxDecoration(
          color: kAccent,
          borderRadius: BorderRadius.circular(1),
        ),
      );

  Widget _card(Note n) {
    final title = n.title.trim().isEmpty ? 'Untitled' : n.title;
    final preview = n.body.trim().isEmpty
        ? 'No additional text'
        : n.body.trim().split('\n').first;
    return Padding(
      padding: const EdgeInsets.only(bottom: 12),
      child: Material(
        color: kSurface,
        borderRadius: BorderRadius.circular(16),
        child: InkWell(
          borderRadius: BorderRadius.circular(16),
          onTap: () => _open(n),
          child: Container(
            padding: const EdgeInsets.fromLTRB(18, 15, 40, 15),
            decoration: BoxDecoration(
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: kLine),
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withOpacity(0.05),
                  blurRadius: 16,
                  offset: const Offset(0, 4),
                ),
              ],
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  title,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                    color: kInk,
                  ),
                ),
                const SizedBox(height: 3),
                Text(
                  preview,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: const TextStyle(fontSize: 13, color: kInk2),
                ),
                const SizedBox(height: 3),
                Text(
                  _relTime(n.updated),
                  style: const TextStyle(fontSize: 11, color: kInk3),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildEditor() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 16, 0),
          child: Row(
            children: [
              GestureDetector(
                onTap: _back,
                child: const SizedBox(
                  width: 44,
                  height: 44,
                  child: Icon(Icons.chevron_left, size: 30, color: kInk),
                ),
              ),
              const Spacer(),
              GestureDetector(
                onTap: _delete,
                child: Container(
                  width: 88,
                  height: 34,
                  alignment: Alignment.center,
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(17),
                    border: Border.all(color: kLine),
                  ),
                  child: const Text(
                    'Delete',
                    style: TextStyle(
                      fontSize: 13,
                      fontWeight: FontWeight.w600,
                      color: kDanger,
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(24, 12, 24, 0),
          child: TextField(
            controller: _titleCtrl,
            focusNode: _titleFocus,
            maxLines: 1,
            style: const TextStyle(
              fontSize: 26,
              fontWeight: FontWeight.w700,
              color: kInk,
            ),
            cursorColor: kInk,
            decoration: const InputDecoration(
              isDense: true,
              border: InputBorder.none,
              hintText: 'Title',
              hintStyle: TextStyle(
                fontSize: 26,
                fontWeight: FontWeight.w700,
                color: kInk3,
              ),
            ),
            onChanged: (_) => _touch(),
          ),
        ),
        const Padding(
          padding: EdgeInsets.fromLTRB(24, 14, 24, 0),
          child: Divider(height: 1, thickness: 1, color: kLine),
        ),
        Expanded(
          child: Padding(
            padding: const EdgeInsets.fromLTRB(24, 10, 24, 0),
            child: TextField(
              controller: _bodyCtrl,
              focusNode: _bodyFocus,
              maxLines: null,
              expands: true,
              textAlignVertical: TextAlignVertical.top,
              style: const TextStyle(fontSize: 16, height: 1.4, color: kInk),
              cursorColor: kInk,
              decoration: const InputDecoration(
                isDense: true,
                border: InputBorder.none,
                hintText: 'Start writing…',
                hintStyle: TextStyle(fontSize: 16, color: kInk3),
              ),
              onChanged: (_) => _touch(),
            ),
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(24, 0, 24, 12),
          child: Text(
            _status,
            style: const TextStyle(fontSize: 11.5, color: kInk3),
          ),
        ),
      ],
    );
  }
}
