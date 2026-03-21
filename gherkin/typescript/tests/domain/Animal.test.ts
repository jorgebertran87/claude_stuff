import { Animal, AnimalSpecies } from "../../src/domain/Animal";

describe("Animal Behavior", () => {
  describe("A hungry animal eats food", () => {
    it.each([
      { species: "Dog" as AnimalSpecies, food: "bone", emotion: "ok", energy: 2 },
      { species: "Cat" as AnimalSpecies, food: "fish", emotion: "satisfied", energy: 5 },
      { species: "Duck" as AnimalSpecies, food: "shit", emotion: "sad", energy: -10 },
    ])(
      "$species eating $food should feel $emotion and gain $energy energy",
      ({ species, food, emotion, energy }) => {
        const animal = new Animal(species);
        expect(animal.state).toBe("hungry");

        animal.feed();

        expect(animal.emotion).toBe(emotion);
        expect(animal.energy).toBe(energy);
      }
    );
  });

  describe("Different animals make different sounds", () => {
    it.each([
      { species: "Dog" as AnimalSpecies, sound: "Woof" },
      { species: "Cat" as AnimalSpecies, sound: "Meow" },
      { species: "Duck" as AnimalSpecies, sound: "Quack" },
    ])("$species should make a $sound sound", ({ species, sound }) => {
      const animal = new Animal(species);
      expect(animal.makeSound()).toBe(sound);
    });
  });
});
