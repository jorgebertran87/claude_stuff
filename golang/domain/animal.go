package domain

type Species string
type Emotion string

const (
	Dog  Species = "Dog"
	Cat  Species = "Cat"
	Duck Species = "Duck"
)

const (
	EmotionOk        Emotion = "ok"
	EmotionSatisfied Emotion = "satisfied"
	EmotionSad       Emotion = "sad"
)

type foodReaction struct {
	food   string
	emotion Emotion
	energy int
}

var foodReactions = map[Species]foodReaction{
	Dog:  {food: "bone", emotion: EmotionOk, energy: 2},
	Cat:  {food: "fish", emotion: EmotionSatisfied, energy: 5},
	Duck: {food: "shit", emotion: EmotionSad, energy: -10},
}

var sounds = map[Species]string{
	Dog:  "Woof",
	Cat:  "Meow",
	Duck: "Quack",
}

type Animal struct {
	Species Species
	state   string
	emotion *Emotion
	energy  int
}

func NewAnimal(species Species) *Animal {
	return &Animal{
		Species: species,
		state:   "hungry",
	}
}

func (a *Animal) State() string {
	return a.state
}

func (a *Animal) Emotion() *Emotion {
	return a.emotion
}

func (a *Animal) Energy() int {
	return a.energy
}

func (a *Animal) Feed(food string) {
	reaction := foodReactions[a.Species]
	if reaction.food != food {
		return
	}
	a.emotion = &reaction.emotion
	a.energy += reaction.energy
	a.state = "fed"
}

func (a *Animal) MakeSound() string {
	return sounds[a.Species]
}
