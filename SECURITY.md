# Security policy

*[Español abajo](#política-de-seguridad)*

RCleaner deletes files and can run as administrator, so security bugs matter a
lot. Examples: a cleaner that can delete something outside its folders, a way
to make the app follow a link out of a cleaned folder, a path the safety checks
should block but don't, or anything that lets another program use RCleaner's
administrator rights.

## Reporting a vulnerability

**Please don't open a public issue.** Use GitHub's private reporting instead:
**Security → Report a vulnerability** on the repository page. Include:

- what can happen and how bad it is,
- steps or a small example to reproduce it,
- the RCleaner version and your Windows version.

You'll get an answer within a few days. We'll work on a fix with you, publish it
in a release and credit you in the notes unless you prefer otherwise.

## Supported versions

Only the latest release gets security fixes.

---

## Política de seguridad

RCleaner borra archivos y puede ejecutarse como administrador, así que los
fallos de seguridad importan mucho: un limpiador que pueda borrar fuera de sus
carpetas, seguir un enlace fuera de una carpeta limpiada, una ruta que las
protecciones deberían bloquear y no bloquean, o cualquier cosa que deje a otro
programa usar los permisos de administrador de RCleaner.

**No abras un issue público.** Usa el reporte privado de GitHub:
**Security → Report a vulnerability** en la página del repositorio, con qué
puede pasar, cómo reproducirlo y tus versiones de RCleaner y Windows.
Responderemos en pocos días y te daremos crédito en la versión que lo corrija,
salvo que prefieras que no.
