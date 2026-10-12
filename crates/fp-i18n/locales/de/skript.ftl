# Meldungen von fp-script (JSF-Laufzeit, automatische Formen, Dateien) – Agent AU
err-script-autoshape-runtime = automatische Formen ({ $part }): { $error }
err-script-autoshape = automatische Form: { $error }
err-script-autoshape-script = Skript der automatischen Form ({ $operation }): { $error }
err-script-shape-missing = shape fehlt (Name der automatischen Form)
err-script-shape-unknown = unbekannte automatische Form „{ $name }“ – autoShape.list zeigt alle
err-script-no-layer = keine Ebene zum Einfügen (Ebene gesperrt oder Webebene aktiv?)
err-script-param-missing = { $param } fehlt
err-script-params-missing = params fehlt (Objekt Schlüssel → Wert)
err-script-not-autoshape = keine automatische Form
err-script-control-point = Steuerpunkt { $index } gibt es nicht (Form hat { $count })
err-script-op-unknown = unbekannter Vorgang „{ $op }“ ({ $ops })
err-script-format-needs-io = Format .{ $ext } wird ohne fp-io nicht gelesen
err-script-png-only = { $path }: nur PNG ohne fp-io
err-script-page-empty = Seite ist leer oder fehlt
err-script-export-unsupported = Exportformat .{ $ext } wird nicht unterstützt
err-script-export-needs-feature = Exportformat .{ $ext } braucht fp-export (Feature „formats“)
err-script-open-failed = Öffnen fehlgeschlagen
err-script-page-missing = Seite fehlt

# >>> UE-3
# UE-3
err-script-element-not-autoshape = Element { $id } ist keine automatische Form
# <<< UE-3

# >>> UE-3 HR
# Texte zu JSF-Fehlercodes (Errors.*), UE-3
err-script-hr-ebad-param = Ein Parameter ist ungültig.
err-script-hr-ebad-param-type = Ein Parameter hat den falschen Typ.
err-script-hr-eout-of-mem = Nicht genügend Arbeitsspeicher.
err-script-hr-efile-not-found = Die Datei wurde nicht gefunden.
err-script-hr-edisk-full = Der Datenträger ist voll.
err-script-hr-egeneric-error-occurred = Es ist ein Fehler aufgetreten.
err-script-hr-eno-sliceable-elems = Es gibt keine Objekte, aus denen Segmente erstellt werden können.
err-script-hr-esharing-violation = Die Datei wird von einem anderen Programm verwendet.
err-script-hr-eunknown-reader-format = Das Dateiformat ist unbekannt.
err-script-hr-ebad-file-contents = Der Dateiinhalt ist beschädigt.
err-script-hr-efile-is-read-only = Die Datei ist schreibgeschützt.
err-script-hr-eno-files-selected = Es wurden keine Dateien ausgewählt.
err-script-hr-ebad-selection = Die Auswahl passt nicht zu diesem Vorgang.
err-script-hr-ebad-js-version = Die Skriptversion wird nicht unterstützt.
err-script-hr-eno-active-document = Es ist kein Dokument geöffnet.
err-script-hr-eno-active-selection = Es ist nichts ausgewählt.
err-script-hr-could-not-complete-your-request = Die Anforderung konnte nicht ausgeführt werden.
err-script-hr-internal = Es ist ein interner Fehler aufgetreten.
# <<< UE-3 HR
# JSF-Laufzeit: nicht umgesetzter API-Member (UE-5)
err-script-not-implemented = nicht implementiert: { $member }
