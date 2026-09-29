# Guion de demo en vivo — O2A

**Versión:** plan actual, con ADR-0008, ADR-0009, ADR-0010 y el ensayo 3
**Audiencia de este documento:** equipo comercial
**Estado del proyecto:** demostración. No es un lanzamiento. No es producto. Mainnet está fuera de alcance.

La red del evento en vivo espera una decisión: `[red del evento: autorización explícita del maintainer]`. Signet es la red del ensayo. Este texto no fija la red del evento.

Cada corchete es una decisión que el maintainer todavía no ha tomado. El texto dentro es un ejemplo, no la decisión. Este guion no escribe el nombre real de ningún artista.

---

## 1. Qué vamos a demostrar, en una frase

> El primer EntityID de O2A se acuña en vivo. El sello ya está financiado, con 6 confirmaciones. En el escenario se firma la génesis, el EntityID aparece de inmediato, dos verificadores dicen CURRENT, el artista firma el nombre oficial y se restaura la copia de respaldo.

Lo que el público debe entender al salir:

- La identidad la controla el artista con sus propias llaves.
- El nombre oficial es una declaración firmada. Firmarla no gasta una transacción de Bitcoin.
- Dos verificadores, en dos laptops, llegan a la misma palabra: CURRENT.

En el escenario entra esto:

| En el escenario | Fuera del escenario |
| --- | --- |
| Génesis firmada delante del público. El EntityID aparece al firmar. | Rotación, recuperación y revocación. ADR-0009 las deja fuera hasta que el programa RGB sea final. |
| El claim `official_name`. | Una atestación. No está en el conjunto congelado. |
| Dos verificadores dicen CURRENT. | Una tercera identidad para un venue o un promotor. |
| Restaurar el paquete desde la copia de respaldo. | Gastar el sello. |

El sello se financia al menos 48 horas antes. En el ensayo 3, las 6 confirmaciones tardaron 38 minutos y 35 segundos. Por eso la espera ocurre antes de que entre el público. Con el sello ya profundo, la firma, las dos comprobaciones, el nombre y la restauración cupieron en menos de 2 segundos de reloj de programa.

Lo que el público también debe oír, en voz alta, antes de crear la identidad: es de prueba y se descarta.

---

## 2. Glosario para hablar con el público

| Término | Cómo lo decimos |
| --- | --- |
| EntityID | El identificador de esta génesis. Aparece en cuanto se firma. |
| Sello | La salida de Bitcoin ya confirmada que la génesis nombra. Su identificador de salida forma parte del EntityID. |
| CURRENT | La palabra del verificador cuando Bitcoin, RGB y O2A coinciden. |
| Nombre oficial | El claim `official_name`. Lo firma el controlador. No es una transacción de Bitcoin. |
| Raíz | La llave que firma la génesis. Después no opera la identidad. |
| Controlador | La llave que firma el nombre oficial. |
| Recuperación | 2 de 3, después de una demora. No se ejecuta en el escenario. |
| Signet | La red del ensayo. Las monedas de prueba no tienen valor. |
| RGB | El carrier de las transiciones futuras, cuando el programa sea final. Hoy el escenario usa la génesis. |

---

## 3. Decisiones

| # | Decisión | Estado |
| --- | --- | --- |
| D1 | Sujeto de la identidad | `[sujeto: sin nombre real]` |
| D2 | Qué se firma en vivo | Fijo. El claim `official_name`. |
| D3 | Quién verifica en la sala | `[verificadores: dos laptops nuestras, o nuestras laptops más un verificador público en los teléfonos del público]` |
| D4 | Duración del segmento en vivo | `[duración del segmento]` |
| D5 | Qué se prepara antes del evento | Fijo. El sello se financia al menos 48 horas antes y llega a 6 confirmaciones. La génesis se firma en el escenario. |
| D6 | Formato | `[formato del evento]` |
| D7 | Segundo actor en escena | `[segundo actor: ninguno]` |

D3 tiene dos opciones abiertas. Una es dos laptops nuestras. La otra es nuestras laptops más un verificador público en los teléfonos del público. Un verificador de un tercero, si se conserva, se nombra como un verificador independiente.

D7 queda como ninguno hasta que alguien lo cambie. Sigue siendo una decisión abierta.

La red del evento es la decisión de la primera línea de este documento. Signet queda como red de ensayo.

Regla que no se negocia: antes de crear cualquier identidad se le dice al participante que es de prueba y se va a descartar. Esto se dice en voz alta.

---

## 4. Los tres actos

El sello ya está financiado y tiene 6 confirmaciones antes de que se abra la puerta. El escenario no espera bloques.

### Acto 1 — Aparece el primer EntityID

**Lo que ve el público**

1. Se presenta el sujeto de D1, sin leer un nombre real que no esté en el plan público.
2. El artista confirma el sello y la política en su dispositivo y firma la génesis.
3. El EntityID aparece en la pantalla en ese momento. También aparece el código QR de ese identificador.

**Lo que pasa por debajo**

- El sello lleva al menos 48 horas financiado y 6 confirmaciones.
- La firma usa el adaptador rgb-protocol 0.11.1, con cierre Opret.
- `signer_entity` son 32 bytes en cero. La raíz firma. El EntityID es el hash etiquetado de esa génesis.

**Frase clave:** "Nadie le dio esta identidad. La firmó con sus llaves, y el identificador aparece ahora."

### Acto 2 — Dos verificadores dicen CURRENT

**Lo que ve el público**

1. Dos personas, en dos laptops, reciben el paquete público. Ninguna tiene la semilla.
2. Las dos dicen CURRENT. La pantalla muestra las dos líneas.
3. Si las dos líneas no coinciden, las dos se quedan visibles.

**Lo que pasa por debajo**

- Cada laptop corre la comprobación del paquete contra Bitcoin.
- Lo ideal es que cada laptop use un backend de Bitcoin distinto.
- El ensayo 3 hizo las dos comprobaciones en dos carpetas de una sola máquina. El escenario usa dos laptops.

**Frase clave:** "No tienen que confiar en nosotros. Las dos máquinas leen el mismo paquete."

### Acto 3 — El nombre oficial y la copia de respaldo

**Lo que ve el público**

1. El artista firma el nombre oficial con la llave de control.
2. La pantalla muestra ese nombre junto al EntityID.
3. En otra máquina, sin la semilla, se restaura el paquete. El mismo EntityID vuelve a salir CURRENT.

**Lo que pasa por debajo**

- El claim es un objeto firmado. No se transmite una transacción de Bitcoin para el nombre.
- Hay dos copias del paquete: una se la lleva el artista y la otra queda con el operador de respaldo.
- Ahí termina el escenario. No hay otra firma.

**Frase clave:** "El nombre es su declaración. La identidad es el EntityID. La copia se puede comprobar sin la semilla."

---

## 5. Plan B por acto

| Momento | Falla | Qué se hace | Qué se dice |
| --- | --- | --- | --- |
| Antes del escenario | El sello no tiene 6 confirmaciones | No se llama final. La página no dice CURRENT. Si hace falta fee, se usa un hijo que pague por el padre. El identificador de la transacción no cambia. | "El pago es el mismo. Seguimos esperando confirmaciones." |
| Acto 1 | El dispositivo falla antes de firmar | Se detiene. No se inventa otra identidad en el escenario. | "No inventamos una segunda identidad aquí." |
| Acto 1 | El dispositivo falla después de firmar | Se muestra el EntityID solo si el archivo firmado ya salió del dispositivo. | "Mostramos el identificador del archivo firmado, o esperamos." |
| Acto 2 | Los dos verificadores no coinciden | Se dejan las dos líneas en pantalla. | "Las dos comprobaciones no coinciden. No lo llamamos final." |
| Acto 2 | Cae la red de la sala | La firma ya está en el archivo. Los verificadores comprueban cuando ven un backend de signet. | "La firma está en el archivo. La comprobación pública sigue cuando vemos la cadena." |
| Acto 3 | Falla la restauración | Se conservan las dos copias y se le dice al artista antes de que se vaya. | "Conservamos las dos copias." |

---

## 6. El caso, en el lenguaje del equipo comercial

`[dolores del mercado que el equipo comercial confirme]`

- **Promotores falsos.** El artista firma el nombre oficial de esta identidad, o no lo firma. Si no firmó, no hay claim, y eso se puede comprobar sin llamar a nadie.
- **Suplantación.** La identidad son llaves que el artista controla. En este escenario no se rota una llave. La recuperación existe y se explica en las preguntas.
- **Una sola declaración comprobable.** El público se lleva el EntityID y el nombre firmado. Un segundo actor no forma parte del plan fijo.
- **Portabilidad.** El paquete se comprueba en otra máquina. Esa comprobación no pide permiso a un servidor nuestro.

Lo que el equipo comercial no promete con este guion:

- Una fecha de producción.
- Integración con la venta de boletos.
- Que la identidad de la demo sea la identidad definitiva del artista.
- Una rotación en vivo, una atestación en vivo, o una red de evento ya elegida.

---

## 7. Preguntas que van a hacer

| Pregunta | Respuesta corta |
| --- | --- |
| ¿Esto es cripto o tokens? | No hay token. Bitcoin fija el sello del EntityID y el orden de esa salida. |
| ¿Cuánto cuesta cada firma? | El nombre oficial no cuesta una transacción de Bitcoin. Es un objeto firmado. El sello se paga antes, con al menos 48 horas y 6 confirmaciones. En el ensayo las monedas son de signet y no tienen valor. |
| ¿Por qué RGB? | RGB es el carrier de las transiciones, cuando el programa sea final. Hoy el escenario firma la génesis y un claim. El claim no paga una transacción. El sello de Bitcoin es el ancla del EntityID. |
| ¿Y si el artista pierde la llave? | La recuperación es 2 de 3, con demora. La oferta por defecto es 1008 bloques. Hacen falta dos de las tres llaves de recuperación. Cualquiera de esas dos puede actuar solo después de la demora. La recuperación no se hace en el escenario. |
| ¿Por qué no una base de datos nuestra? | Porque entonces hay que confiar en nosotros. Las dos laptops leen el mismo paquete. |
| ¿Cuándo en producción? | No hay fecha. ADR-0009 deja las transiciones fuera hasta que el programa RGB sea final. Esta demo no es un lanzamiento. |

---

## 8. Checklist previo al evento

- [ ] El sello está financiado.
- [ ] El sello tiene 6 confirmaciones.
- [ ] Ninguna entrada de esa transacción señala reemplazo.
- [ ] Hay dos laptops de verificación. Lo ideal es que usen backends de Bitcoin distintos.
- [ ] Hay dos copias de respaldo: una para el artista y una para el operador de respaldo.
- [ ] El participante oyó que la identidad es de prueba y se descarta.
- [ ] La semilla no está en la laptop del operador ni en las laptops de los verificadores.
- [ ] La red del ensayo es signet. La red del evento sigue abierta en la decisión de la primera sección.

---

## 9. Quién decide qué

| Decisión abierta | Quién la cierra | Fecha |
| --- | --- | --- |
| D1, D3, D4, D6, D7 y la red del evento | `[maintainer, con el equipo comercial en D1, D4, D6 y D7]` | `[fecha límite]` |

D2 y D5 ya están fijas. Con las filas abiertas resueltas, el escenario tiene sujeto, verificadores, duración, formato, segundo actor y red.
