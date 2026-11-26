# Traducir RCleaner

[English](TRANSLATING.md) · Español

Todos los textos de RCleaner (la app, los nombres y descripciones de los
limpiadores, los mensajes de error y la línea de comandos `rcl`) están en un
archivo JSON por idioma en [`locales/`](../locales). La app y el motor en Rust
leen los mismos archivos, así que un idioma añadido ahí aparece en todas partes.
No hace falta saber Rust ni Svelte.

Idiomas disponibles: **inglés** (`en.json`, la referencia) y **español**
(`es.json`).

## Añadir un idioma

1. Copia `locales/en.json` a `locales/<código>.json` con el
   [código del idioma](https://es.wikipedia.org/wiki/ISO_639-1): `fr.json`,
   `pt.json`, `de.json`… Usa región sólo si de verdad cambia: `pt-BR.json`.
2. Cambia `_meta.name` por el nombre del idioma en ese idioma: `"Français"`,
   `"Português"`.
3. Traduce los **valores**. Nunca cambies las **claves** (lo de la izquierda).
4. Compruébalo (ver abajo) y abre un pull request llamado `i18n(<código>): …`.

Nada más: no hay código que tocar. La app muestra todos los archivos de
`locales/` en *Ajustes → Idioma* y elige el tuyo sola si coincide con el idioma
de Windows.

No hace falta terminarlo de una vez. Las claves que falten se muestran en
inglés, así que una traducción parcial ya sirve. Borra las líneas que no hayas
traducido en lugar de dejar texto en inglés en tu archivo; así se ve fácilmente
qué falta.

## Reglas para los valores

- **Conserva las `{variables}`** tal cual: `"{size} free"` → `"{size} libres"`.
  Puedes moverlas dentro de la frase, pero no renombrarlas, traducirlas ni
  quitarlas.
- **Plurales** con dos claves: `_one` para exactamente 1 y `_other` para el resto.

  ```json
  "files_one": "{n} archivo",
  "files_other": "{n} archivos"
  ```

  Si en tu idioma la palabra no cambia, pon el mismo texto en las dos.
- No traduzcas **marcas ni nombres técnicos**: Windows, Steam, `node_modules`,
  `%TEMP%`, `pip cache purge`, rutas y ejemplos de comandos entre comillas
  invertidas.
- Usa el **tono de la app**: corto, claro y cercano; háblale al usuario como lo
  hacen normalmente las apps en tu idioma.
- Usa la tipografía de tu idioma (comillas «», puntos suspensivos `…`).
- Mantén el JSON válido: comillas dobles, comas entre entradas y `\\` para una
  barra invertida (`"C:\\Windows\\Temp"`).

## Cómo se organizan las claves

| Sección | Qué contiene |
|---|---|
| `ui.*` | Todo lo que ves en las ventanas de la app |
| `module.*` | Nombres (`name`) y descripciones (`desc`) de los limpiadores y las filas que generan (`target`, `detail`) |
| `mode.*` | Nombres y descripciones de los modos incluidos |
| `category.*`, `risk.*` | Nombres de grupos y niveles de riesgo |
| `error.*`, `app.*`, `core.*` | Mensajes de error y avisos |
| `cli.*` | Salida de la línea de comandos `rcl` |

## Comprobar la traducción

```powershell
cargo test -p crp-core i18n
```

El test falla si el archivo no es JSON válido, si tiene una clave que no existe
en `en.json` o si una `{variable}` no coincide con la del inglés. Después abre
la app (`cd app; cargo tauri dev`), elige tu idioma en *Ajustes → Idioma* y
busca textos cortados o que no quepan.

Si no puedes compilar el proyecto, abre el pull request igualmente: la CI hace
la misma comprobación.

## Mantener las traducciones al día

Quien añade una función añade sus claves a `en.json` (y a `es.json`). Los demás
idiomas muestran esas claves en inglés hasta que alguien las traduce. Los issues
con la etiqueta `i18n` dicen qué falta.
