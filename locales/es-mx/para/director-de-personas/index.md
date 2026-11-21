# Para el Director de Personas

> Resumen técnico, no es asesoría legal, certificación ni auditoría. Indica lo que
> muestra el repositorio al 2026-10-11. Nada de lo aquí escrito afirma una acreditación.

## Por qué importan las personas en este producto

El acceso a los datos personales se decide por rol. El software aplica lo que dice el
token de una persona; **quién recibe qué rol y cuándo se le retira es un proceso
de personas** que este repositorio no puede hacer por usted.

## Qué ofrece el software a sus procesos

- **Visibilidad por rol:** algunos roles ven valores enmascarados en los canales de contacto,
  los montos de negociaciones y las vistas de dinero; gerentes, administradores y niveles de servicio ven más
  ([auth](../../../../spec/auth.md)).
- **Lecturas auditadas:** leer datos personales deja un rastro que nombra a la persona.
  Hay que informarlo al personal antes de la puesta en marcha.
- **Acceso de emergencia:** una persona puede tomar una elevación de lectura justificada de hasta 60
  minutos; otra persona debe revisarla después. Capacite a ambas partes.
- **Cierre de sesión por inactividad** tras 15 minutos en el navegador.
- **Accesibilidad:** una declaración pública y verificaciones automáticas; la auditoría manual
  (T48) aún requiere usuarios con discapacidad y probadores de tecnología de asistencia.
- **Idiomas:** la interfaz está traducida a 8 configuraciones regionales; las traducciones
  **no** han sido revisadas por hablantes nativos.

## Sus responsabilidades

| Área | Acción |
|---|---|
| Altas, cambios y bajas | Otorgue y retire roles mediante su proveedor de identidad; el CRM rechaza los tokens vencidos pero no administra cuentas |
| Capacitación | Protección de datos, reglas de consentimiento, uso y revisión del acceso de emergencia, prácticas accesibles |
| Políticas | Uso aceptable, aviso de monitoreo (las lecturas se auditan), ruta disciplinaria por mal uso del acceso de emergencia |
| Capacidad de revisión | Designe quién revisa las concesiones de acceso de emergencia y quién cubre cuando esa persona no esté |
| Reclutamiento | Roles que necesita el programa: enlace con el probador independiente, responsable de seguridad clínica si aplica el ámbito del NHS, responsable de protección de datos, responsable de accesibilidad |
| Bienestar | Apoyo al personal que atiende casos sensibles; el CRM no es un sistema de gestión de casos, pero puede guardar datos sensibles de relaciones |

## Pendientes que requieren personas

Auditoría manual de accesibilidad con usuarios de tecnología de asistencia (T48), revisión de
las traducciones por hablantes nativos y asignación de responsables de las tareas de revisión y alertas mencionadas.
Consulte [tasks.md](../../../../spec/tasks.md).
