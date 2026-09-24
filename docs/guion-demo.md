# Guion de demo en vivo — O2A sobre Bitcoin signet

**Versión:** plantilla para decisión · **Audiencia de este documento:** equipo comercial
**Estado del proyecto:** demostración en red de prueba. No es producto. No es mainnet.

Cada `[corchete]` es una decisión pendiente. El texto dentro es un ejemplo, no la decisión.
Cuando todos los corchetes estén resueltos, el guion pasa a ingeniería para construir el milestone 2.

---

## 1. Qué vamos a demostrar, en una frase

> Un artista crea una identidad que no depende de ninguna plataforma, la usa para firmar
> `[una atestación de un evento]`, y dos verificadores que no confían entre sí llegan al mismo
> resultado usando solo Bitcoin.

Lo que el público debe entender al salir:

- La identidad la controla el artista con sus propias llaves, no nosotros ni nadie.
- Lo que firma el artista se puede verificar sin pedirle permiso a ninguna empresa.
- Bitcoin pone el orden y el sello; nadie puede reescribir la historia.

Lo que **no** vamos a afirmar:

- Que esto está en producción o en mainnet.
- Que las identidades creadas hoy persisten (son de prueba y se descartan).
- Que reemplaza el sistema de ticketing actual.

---

## 2. Glosario para hablar con el público

| Término técnico | Cómo lo decimos en la demo |
|---|---|
| EntityID | La identidad del artista: un identificador único que solo él controla |
| Controlador | La llave con la que el artista firma hoy; se puede rotar sin perder la identidad |
| Atestación | Una declaración firmada por el artista: "yo confirmo X" |
| Sello / anclaje en Bitcoin | El momento en que la declaración queda fijada en Bitcoin y ya no se puede alterar |
| Paquete de prueba | El archivo que cualquiera puede verificar sin conectarse a nosotros |
| Verificador | Un programa independiente que dice "válido" o "inválido" sin consultar a nadie |
| Signet | Red de prueba de Bitcoin: bloques cada 10 minutos, bitcoins sin valor |

---

## 3. Decisiones que definen la demo

| # | Decisión | Opciones | Elegido |
|---|---|---|---|
| D1 | Sujeto de la identidad | artista real participante / artista ficticio con nombre de fantasía | `[artista real: nombre]` |
| D2 | Qué se atesta en vivo | un evento (fecha, venue) / un ticket / un lanzamiento (álbum) | `[evento: nombre, fecha, venue]` |
| D3 | Quién verifica en la sala | dos laptops nuestras / una nuestra + una de RGB-WG / el público con explorador público | `[una nuestra + una de RGB-WG]` |
| D4 | Duración del segmento en vivo | 8 / 12 / 20 minutos | `[12 minutos]` |
| D5 | Qué se pre-ancla antes del evento | la génesis de la identidad / génesis + primera rotación / nada | `[génesis pre-anclada]` |
| D6 | Formato | escenario con pantalla / mesa de trabajo con invitados / video grabado con respaldo en vivo | `[escenario]` |
| D7 | Segundo actor en escena | venue real / promotor / ninguno | `[venue: nombre]` |

Regla que no se negocia: **antes de crear cualquier identidad se le dice al participante que es
de prueba y se va a descartar.** Esto se dice en voz alta en la demo.

---

## 4. Los tres actos

### Acto 1 — La identidad existe y es del artista (≈ `[3]` min)

**Lo que ve el público**

1. Presentamos a `[artista, D1]`. Explicamos en una frase qué es una identidad autocustodiada.
2. En pantalla: la identidad ya existe en signet (pre-anclada, D5). Mostramos el identificador
   y el bloque de Bitcoin donde quedó sellada, en un explorador público.
3. `[opcional]` El artista rota su llave de control en vivo: "cambié de teléfono, sigo siendo yo".
   La transacción entra a la mempool; el público la ve aparecer en el explorador.

**Lo que pasa por debajo**

- Génesis del contrato RGB de identidad anclado en signet días antes.
- Rotación de controlador = transición RGB + objeto O2A firmado, anclados en una transacción.

**Frase clave:** "Nadie le dio esta identidad. La derivó de sus propias llaves."

---

### Acto 2 — El artista firma algo que importa (≈ `[5]` min)

**Lo que ve el público**

1. `[artista]` firma `[la atestación del evento, D2]`: "confirmo que me presento en
   `[venue, D7]` el `[fecha]`".
2. `[si D7 = venue]` El venue, con su propia identidad, firma una contra-atestación:
   "confirmo que `[artista]` está en cartelera ese día".
3. Se genera el **paquete de prueba**: un archivo. Lo mostramos como lo que es, un archivo que
   se puede mandar por correo, imprimir como QR o guardar.
4. La transacción que ancla la atestación se transmite. Mempool visible en el explorador.

**Lo que pasa por debajo**

- Objeto O2A `[attestation / event manifest]` con encoding canónico, firmado por el controlador.
- Anclaje en signet; el sello del RGB queda cerrado.

**Frase clave:** "Esto no vive en nuestros servidores. Vive en un archivo y en Bitcoin."

**Tiempo muerto esperado:** entre transmitir y confirmar pasan hasta 10 minutos. Aquí va la
explicación de por qué importa (fraude de promotores, artistas suplantados, reventa sin
trazabilidad). Ver sección 6.

---

### Acto 3 — Dos verificadores que no se conocen dicen lo mismo (≈ `[4]` min)

**Lo que ve el público**

1. El paquete de prueba se entrega a `[los dos verificadores, D3]`. Ninguno tiene acceso a
   nuestra infraestructura; solo tienen el archivo y su propia conexión a Bitcoin.
2. Los dos corren la verificación. En pantalla, lado a lado, el mismo resultado:
   - Bitcoin: anclado en bloque `[N]`, sello gastado correctamente.
   - RGB: historia válida.
   - O2A: autorización válida, firma del controlador vigente.
3. `[opcional, si sobra tiempo]` Mostramos un paquete alterado: un byte cambiado. Ambos
   verificadores lo rechazan.

**Lo que pasa por debajo**

- Tres capas de verificación independientes que nunca se colapsan en una.
- El verificador no consulta ninguna API nuestra durante la evaluación.

**Frase clave:** "No tienen que confiar en nosotros. Tienen que confiar en Bitcoin y en las
matemáticas, y eso ya lo hacen."

---

## 5. Plan B por acto

| Falla posible | Respuesta preparada |
|---|---|
| La transacción no confirma en el tiempo del segmento | Mostrar la mempool (ya es prueba de transmisión) y una atestación confirmada el día anterior |
| Cae la conexión del venue | El nodo signet corre local en la laptop; la verificación del Acto 3 funciona igual. Solo se pierde la vista del explorador público |
| Falla la laptop principal | Nodo de respaldo en `[GCP, VM pequeña]` sincronizado; video grabado del segmento completo |
| Un verificador da resultado distinto | No se improvisa. Se detiene la demo y se dice que se investiga. Un resultado distinto es exactamente lo que el sistema debe detectar |

---

## 6. El caso, en el lenguaje del equipo comercial

`[Ajustar a los dolores reales que el equipo ve en el mercado]`

- **Promotores falsos.** Hoy cualquiera anuncia un show con el nombre de un artista. Con O2A,
  el artista firma o no firma. Si no firmó, no hay atestación, y eso se puede comprobar en
  segundos sin llamar a nadie.
- **Suplantación en redes y venta directa.** La identidad del artista no es una cuenta que se
  hackea; son llaves que él controla y puede rotar.
- **Trazabilidad para venues y promotores.** Cada actor tiene su propia identidad; una
  cartelera se convierte en un conjunto de firmas cruzadas verificables.
- **Portabilidad.** Nada de esto depende de nuestra plataforma. Ese es el argumento, no la
  debilidad: la plataforma vende servicio encima de una capa que el mercado puede verificar.

Lo que el equipo comercial **no** debe prometer todavía:

- Fechas de producción.
- Integración con la venta de tickets actual.
- Que las identidades creadas en la demo sean las definitivas.

---

## 7. Preguntas que van a hacer

| Pregunta | Respuesta corta |
|---|---|
| ¿Esto es cripto / tokens? | No. No hay token. Usamos Bitcoin solo para fijar el orden de los hechos. |
| ¿Cuánto cuesta cada firma? | Una transacción de Bitcoin; hoy en red de prueba, sin costo. En mainnet, centavos, y varias atestaciones pueden ir en una sola. |
| ¿Y si el artista pierde sus llaves? | El protocolo tiene recuperación con umbral y demora. `[No se demuestra hoy]`. |
| ¿Por qué no una base de datos nuestra? | Porque entonces hay que confiar en nosotros. El punto es que no haga falta. |
| ¿Cuándo en producción? | Cuando la librería RGB tenga versión final. Estamos trabajando con su equipo; ya les enviamos correcciones. |

---

## 8. Checklist previo al evento

- [ ] Nodo signet sincronizado con `[7]` días de anticipación; `status.py` sale 0
- [ ] Nodo de respaldo `[GCP]` sincronizado
- [ ] sBTC del faucet confirmados en el wallet de la demo
- [ ] Génesis de `[artista]` y `[venue]` anclados y confirmados (D5)
- [ ] Paquete de prueba del día anterior verificado en ambas máquinas (Plan B)
- [ ] Video de respaldo grabado del segmento completo
- [ ] Participante informado de que la identidad es de prueba (regla no negociable)
- [ ] Explorador público abierto en signet, con la dirección del anclaje a mano
- [ ] Restart policy de bitcoind cambiada a `unless-stopped` solo para el día del evento

---

## 9. Quién decide qué

| Corchete | Decide | Fecha límite |
|---|---|---|
| D1, D2, D7 (sujeto, objeto, segundo actor) | `[comercial + maintainer]` | `[fecha]` |
| D3 (verificadores) | `[maintainer con RGB-WG]` | `[fecha]` |
| D4, D6 (duración, formato) | `[comercial]` | `[fecha]` |
| D5 (pre-anclaje) | ingeniería, según D4 | tras D4 |

Con D1–D7 resueltos, ingeniería tiene lo necesario para el milestone 2: el objeto de
atestación, el comando del CLI y la evidencia de verificación por dos clientes.
