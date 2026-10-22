# 🛡️ Política de Seguridad y Postura Defensiva (`SECURITY.md`)

La seguridad, la integridad de los datos del usuario y la inmunidad frente a ataques en la cadena de suministro son pilares fundamentales del diseño de **Rustext**.

---

## 📌 Versiones Soportadas

| Versión | Estado de Soporte | Dependencias de Terceros |
| :---: | :--- | :---: |
| **`0.2.x`** | ✅ **Soportada activamente** (Parches de seguridad y actualizaciones) | `0` (`std::*` puro) |
| `< 0.2.0` | ⚠️ Actualizar a `v0.2.0` o superior | `0` (`std::*` puro) |

---

## 🚨 Cómo Reportar una Vulnerabilidad

Si descubres una vulnerabilidad de seguridad en **Rustext**, te agradecemos que la reportes de manera responsable:

1. **Contacto directo**: Envía un correo detallado a **boleklolek** (`bolek.lolek@gmail.com`) o utiliza la pestaña [**Security Advisories**](https://github.com/boleklolek/rustext/security/advisories) del repositorio en GitHub.
2. **Información a incluir**:
   - Descripción técnica de la debilidad y su categoría CWE estimada.
   - Pasos exactos o archivo de prueba (*Proof of Concept*) para reproducir el problema.
   - Sistema operativo y modo de ejecución afectado (GUI Wayland/X11/Win32 o TUI).
3. **Tiempo de respuesta**: Confirmaremos la recepción del reporte en un plazo máximo de **48 horas** y publicaremos el parche junto con su identificador `SEC-XX` y test de regresión en `tests/gui_tests.rs`.

---

## 🔒 Arquitectura de Seguridad por Diseño

### 1. Superficie de Ataque 0.0% en Cadena de Suministro (Supply Chain Immunity)
A diferencia de los editores modernos que dependen de cientos de paquetes de `crates.io` o miles de módulos de `npm`, **Rustext tiene cero dependencias externas** en [`Cargo.toml`](Cargo.toml). Esto erradica por completo vectores como:
- *Typosquatting* y paquetes maliciosos en registros públicos.
- Vulnerabilidades transitivas en librerías de terceros desactualizadas.
- Scripts `build.rs` de terceros que ejecutan código arbitrario durante la compilación.

### 2. Seguridad de Memoria en 100% Safe Rust
El motor de edición (*Piece Table*), el renderizador de software (`Framebuffer`), el decodificador de fuentes TrueType (`.ttf`), el motor Flexbox y el cliente LSP están implementados en **Safe Rust**, previniendo estructuralmente desbordamientos de búfer (*buffer overflows*), *use-after-free* y corrupción de heap.

### 3. Ejecución Segura de Subprocesos y Resistencia a ReDoS
- **Sin Intérpretes Shell Intermedios**: Todas las llamadas a herramientas externas (`git`, servidores LSP, formateadores y linters) usan vectores de argumentos directos en `std::process::Command::new()`, impidiendo inyecciones de comandos por metacaracteres de shell (`CWE-78`).
- **Inmunidad a ReDoS (`CWE-1333`)**: Los motores de búsqueda y resaltado léxico operan en tiempo lineal determinista $\mathcal{O}(N)$ sin expresiones regulares vulnerables a retroceso catastrófico.

---

## 📋 Registro Histórico de Auditoría OWASP / CWE

| ID | Severidad | Vulnerabilidad / Vector | CWE | Mitigación Verificada |
| :---: | :---: | :--- | :---: | :--- |
| **SEC-01** | **Media** | Carga de extensiones locales no confiables | [CWE-829](https://cwe.mitre.org/data/definitions/829.html) / [CWE-94](https://cwe.mitre.org/data/definitions/94.html) | Confinamiento estricto a `~/.rustext/extensions` sin ejecución de código dinámico arbitrario. |
| **SEC-02** | **Media** | Guardado no atómico y truncamiento en fallo de disco | [CWE-377](https://cwe.mitre.org/data/definitions/377.html) / [CWE-459](https://cwe.mitre.org/data/definitions/459.html) | Escritura atómica mediante archivo temporal hermano, `sync_all()` a disco y `fs::rename()`. |
| **SEC-03** | **Baja** | Inyección de argumentos CLI en portapapeles Wayland | [CWE-88](https://cwe.mitre.org/data/definitions/88.html) | Canalización directa a `wl-copy` por `stdin`, evitando límites `ARG_MAX` e inyección de flags. |
| **SEC-04** | **Baja** | Recursión infinita por enlaces simbólicos cíclicos | [CWE-674](https://cwe.mitre.org/data/definitions/674.html) / [CWE-59](https://cwe.mitre.org/data/definitions/59.html) | `GlobalSearchEngine` omite enlaces simbólicos y limita la profundidad recursiva a 32 niveles. |
| **SEC-05** | **Baja** | Deserialización de borradores fuera de directorio | [CWE-22](https://cwe.mitre.org/data/definitions/22.html) | Confinamiento estricto a `drafts/`, validación `is_safe_draft_path()` y rechazo de `..`. |
| **SEC-06** | **Info** | Persistencia de modo raw en terminal tras pánico | [CWE-755](https://cwe.mitre.org/data/definitions/755.html) | `std::panic::set_hook` restaura visibilidad del cursor, pantalla principal y `stty sane`. |
| **SEC-07** | **Baja** | Creación predecible de archivos shm temporales | [CWE-377](https://cwe.mitre.org/data/definitions/377.html) / [CWE-59](https://cwe.mitre.org/data/definitions/59.html) | Apertura exclusiva `O_CREAT \| O_EXCL` con permisos UNIX `0600` en `/dev/shm`. |
| **SEC-08** | **Media** | *Path Traversal* al cargar archivos `.theme` externos | [CWE-22](https://cwe.mitre.org/data/definitions/22.html) | Lista blanca `is_safe_theme_name()` (64 bytes máx.) y verificación canónica `starts_with(canonical_base)`. |
| **SEC-09** | **Baja** | Denegación de servicio (DoS) por archivos `.theme` gigantes | [CWE-400](https://cwe.mitre.org/data/definitions/400.html) / [CWE-776](https://cwe.mitre.org/data/definitions/776.html) | Límite estricto `MAX_THEME_FILE_SIZE = 512 KB` y cuota máxima de 256 archivos por carpeta. |
| **SEC-10** | **Baja** | Fuga acumulativa de memoria en Paleta de Comandos | [CWE-401](https://cwe.mitre.org/data/definitions/401.html) | Uso de `Cow<'static, str>` en `CommandItem`, liberando cadenas dinámicas al cerrar el modal. |
