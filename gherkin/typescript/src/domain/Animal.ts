export type AnimalSpecies = "Dog" | "Cat" | "Duck";
export type Emotion = "ok" | "satisfied" | "sad";

const FOOD_REACTIONS: Record<
  AnimalSpecies,
  { food: string; emotion: Emotion; energy: number }
> = {
  Dog: { food: "bone", emotion: "ok", energy: 2 },
  Cat: { food: "fish", emotion: "satisfied", energy: 5 },
  Duck: { food: "shit", emotion: "sad", energy: -10 },
};

const SOUNDS: Record<AnimalSpecies, string> = {
  Dog: "Woof",
  Cat: "Meow",
  Duck: "Quack",
};

export class Animal {
  readonly species: AnimalSpecies;
  private _state: "hungry" | "fed" = "hungry";
  private _emotion: Emotion | null = null;
  private _energy: number = 0;

  constructor(species: AnimalSpecies) {
    this.species = species;
  }

  get state() {
    return this._state;
  }

  get emotion(): Emotion | null {
    return this._emotion;
  }

  get energy(): number {
    return this._energy;
  }

  feed(): void {
    const reaction = FOOD_REACTIONS[this.species];
    this._emotion = reaction.emotion;
    this._energy += reaction.energy;
    this._state = "fed";
  }

  makeSound(): string {
    return SOUNDS[this.species];
  }
}
