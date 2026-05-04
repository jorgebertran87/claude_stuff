---
description: Analiza un tablero de buscaminas y determina movimientos seguros
---

Este es el juego del buscamins. Se te da un JSON con el estado del tablero (fila/columna desde 1, tanto la fila como la columna, JAMÁS EMPIECES A CONTAR POR 0): mines_remaining, flags (banderas ya puestas), unrevealed (celdas sin revelar — las únicas que debes analizar), revealed (celdas descubiertas con valor "empty" o 1-8).

$ARGUMENTS

Determina un único movimiento seguro. Responde en texto plano sin markdown: celda segura para revelar (fila, columna) o celda que es mina con certeza. Si no puedes determinarlo con certeza, dilo.

Básate en estas reglas:

"Reglas
El tablero está dividido en celdas, con minas distribuidas al azar. Para ganar, debes abrir todas las celdas que no contienen minas. Al hacer clic en una celda que no tiene una mina, se revela un número. Este número es la cantidad de celdas vecinas que contienen una mina. Con esta información, puedes determinar las celdas que son seguras y las celdas que contienen minas. Las celdas sospechosas de contener minas se pueden marcar con una bandera usando el botón derecho del ratón.

Para comenzar una partida nueva, puedes hacer clic en la cara feliz que está en la parte superior del tablero o usar la barra espaciadora. El número restante de minas se muestra en la esquina izquierda y el cronómetro del juego se muestra en la esquina derecha.

Chording
Cuando un número tiene la cantidad correcta de banderas, puedes hacer clic en él para abrir todas las celdas que lo rodean. Esto se llama chording (acorde en inglés) porque en versiones anteriores requería presionar dos botones, izquierdo + derecho, al mismo tiempo (se puede cambiar en la configuración). El uso de los chords reduce en gran medida los clics innecesarios y es la base de un juego eficiente.

Modo sin adivinar (NG)
Una de las mejores formas de aprender a jugar es jugar sin adivinar. En este modo, se proporciona una posición inicial y nunca es necesario adivinar para completar el tablero. Si te quedas atascado, hay un botón de pista gratuito en la parte inferior derecha. Los tableros NG de mayor dificultad requieren el uso de patrones lógicos más complejos. La dificultad extremo contiene al menos una situación avanzada en cada juego.

Sin banderas (NF)
NF es un estilo de juego que no usa banderas. Se gana un juego cuando todas las celdas que no son minas se revelan, da igual si las minas están marcadas o no. Los jugadores de NF usan esto para reducir la cantidad de clics necesarios para completar un tablero.

3BV
3BV (Valor de referencia del tablero de Bechtel) es el número mínimo de clics necesarios para completar un tablero sin utilizar banderas. 3BV se utiliza para medir la dificultad relativa de un tablero y la velocidad a la que estás jugando (a menudo expresado como 3BV / s, o 3BV por segundo).

Para evitar juegos muy afortunados, hay límites de 3BV para novato, aficionado y experimentado. Estos son 5/30/100 respectivamente. Los juegos con un 3BV inferior a los límites no se registrarán en la clasificación de tiempo."

NO ALUCINES...BÁSATE ESTRICTAMENTE EN LA INFORMACIÓN PASADA EN EL JSON
