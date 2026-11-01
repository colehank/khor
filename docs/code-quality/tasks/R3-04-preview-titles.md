# R3-04 Preview title consistency

State: implementation and author self-review complete. Root only, no main/ship.

Temporary file/commit names are italic in the shared one/two-line TabStrip, tab switcher, original mobile document deck and content overview. No hollow preview mark. Fixed names and subtitles remain normal; keep remains the existing owner decision. Explicit unread marks remain separate. While verifying the promised dirty mark, the shared TabFace.dirty field was found never consumed; the existing close seat now renders its intended unsaved dot (same close behavior, no new draft owner). This directly completes the scoped dirty/unread contract.

Evidence `/Volumes/disk0/Khor/evidence/round3/R3-04`: real Chrome baseline title was normal (red), then green after typography. A second real two-line dirty regression showed no dirty mark (red). Final combined fixture tests original owners, double-click normal style, typing keeps file, both title lines, narrow layout, unread/dirty marks and original phone deck. Native Mac/iPhone human acceptance remains pending.

Final `final/report.json`:5/5 actual Chrome groups passed at unchanged source hashes; normal/italic computed style, no hollow ring, two-line subtitle normal, dirty/unread marks present, original mobile deck. Final combination unit and types exit0; chat recovery18/18 Chrome passed including original13 history cases. Initial npm test pretest build stopped(exit130), its known new2.7GiB target removed; explicit checks then succeeded. No claim of WebKit/native human acceptance.
