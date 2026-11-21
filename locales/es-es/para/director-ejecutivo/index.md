# Para el Director Ejecutivo

> Resumen técnico, no es asesoría legal, certificación ni auditoría. Indica lo que
> muestra el repositorio al 2026-10-11. Nada de lo aquí escrito afirma una acreditación.

## La decisión en un párrafo

Este es un CRM de nivel demostración con una base de seguridad y privacidad
inusualmente sólida. **No está listo** para guardar datos reales de pacientes o
ciudadanos, y el propio repositorio lo dice ([plan.md](../../../../spec/plan.md)). Adoptarlo es decidir financiar el
trabajo externo pendiente, no ponerlo en marcha.

## Qué existe

- Un CRM funcional que abarca ventas, marketing, soporte y analítica, con una API en Rust
  y una interfaz web, construido conforme a una especificación escrita.
- Marketing que parte del consentimiento, control de acceso que oculta los valores sensibles a
  los roles que no deben verlos, una pista de auditoría que evidencia la manipulación, acceso
  y supresión de datos para solicitudes de privacidad, y acceso de emergencia («break-glass»)
  que se justifica, tiene límite de tiempo y se revisa.
- 54 de 58 elementos de trabajo registrados están terminados ([tasks.md](../../../../spec/tasks.md)).

## Qué se interpone entre esto y el uso real

| Obstáculo | Por qué importa | Quién puede resolverlo |
|---|---|---|
| Sin entrega real de correo (T39) | El marketing es simulado; un proveedor real requiere contratos y un acuerdo de tratamiento de datos | Usted elige al proveedor; Legal firma |
| Sin prueba de seguridad independiente (T42) | Los hospitales y los compradores gubernamentales suelen exigirla; nuestras propias pruebas no son independientes | Una firma externa |
| Sin auditoría manual de accesibilidad (T48) | Los organismos públicos deben publicar una declaración de accesibilidad que puedan respaldar | Personas con tecnología de asistencia |
| La firma de versiones está construida pero no se ha ejecutado en una versión real, y ningún revisor ha comprobado el borrador de estándares de la API (T49) | Las listas de verificación de adquisiciones piden ambas cosas | Ingeniería (etiquetar una primera versión); un revisor externo |

## Decisiones que le corresponden

1. **Qué mercado primero.** La evaluación de los Criterios de Evaluación de Tecnología Digital (DTAC) del NHS
   ([documentos](../../../../docs/united-kingdom/national-health-service/digital-technology-assessment-criteria/2.0/index.md))
   concluye que el producto probablemente queda fuera del alcance de DTAC y no aprobaría
   las secciones aplicables por falta de evidencia organizacional (responsable de seguridad clínica
   designado, certificaciones). Elija la ruta antes de invertir en ella.
2. **Si financiar el aseguramiento independiente** (T42, T48). Sin él, la afirmación
   honesta es «construido con un alto estándar, sin probar por terceros».
3. **Quién es dueño del riesgo** una vez que se guarden datos reales. Consulte los resúmenes del [director de riesgos](../director-de-riesgos/index.md)
   y del [director jurídico](../director-jurídico/index.md).

## Qué no decir externamente

No afirme cumplimiento, certificación, «cumple con HIPAA» ni «aprobado por el NHS».
El paquete de evidencia ([cumplimiento](../../../../spec/compliance/index.md)) respalda una declaración más acotada y
verdadera: los controles están mapeados, los implementados están probados y las brechas están identificadas.
