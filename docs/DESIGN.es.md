# Diseño de la interfaz

[English](DESIGN.md) · Español

Referencias estudiadas: **CleanMyMac** (MacPaw, premios Red Dot / iF por su UI),
**DaisyDisk** (mapa radial de disco) y **Ashampoo WinOptimizer** (lista de
resultados densa y clara en Windows). De cada una se tomó una idea, no su aspecto:

| Referencia | Idea adoptada |
|---|---|
| CleanMyMac | Una herramienta por pantalla, cada una con **su color**, un glifo grande y **un único botón circular** abajo al centro que siempre está en el mismo sitio (Analizar → Detener → Limpiar → Listo). Menú lateral de iconos, corto. |
| DaisyDisk | Mapa **radial** por carpetas, ordenado por tamaño, con lo diminuto agrupado; centro = carpeta actual (clic para subir); lista al lado; "bandeja" para elegir qué eliminar sin borrar al momento. |
| WinOptimizer | Resultados como **filas por categoría** (icono, nombre, nº de elementos, tamaño, desplegar) en vez de tarjetas. Cifra total grande arriba. |

## Reglas

1. **Sin tarjetas con borde por todas partes.** Se separa con espacio y líneas
   finas (`--line`). Sólo los diálogos y los grupos de Ajustes tienen superficie.
2. **Una acción principal por pantalla**, siempre en el `Dock` (botón circular).
   Las acciones secundarias son botones discretos (`.btn.quiet`).
3. **Color por sección** con `[data-section]` → variables `--a` / `--b`:
   Resumen azul, Limpieza verde, Espacio violeta, Duplicados naranja, Ajustes gris.
   El riel, el glifo, el botón, las casillas y el brillo de fondo lo heredan.
4. **Cifras grandes** con la unidad en pequeño (`splitBytes`), números tabulares.
5. **Riesgo como texto con punto de color**, no píldoras.
6. Tipografía nativa: *Segoe UI Variable Display* (títulos) y *Text* (resto).
7. Claro/oscuro según el sistema (o lo que elija el usuario). Animaciones cortas
   (≤ 350 ms) y desactivadas con `prefers-reduced-motion`.

## Piezas

| Componente | Uso |
|---|---|
| `Glyph` | Baldosa con degradado de la sección + icono; anillo giratorio con `busy`. |
| `ActionButton` | Botón circular principal (`go` / `stop`). |
| `Dock` | Barra inferior: izquierda (secundario), centro (botón), derecha (resumen). |
| `Modal` | Diálogo con fondo desenfocado; Escape/clic fuera cierra. |
| `Sunburst` | Mapa radial (`lib/sunburst.ts` calcula arcos y colores). |
| `ResultList` | Filas por categoría plegables con sus hallazgos. |

Tokens en `app/src/app.css`.
