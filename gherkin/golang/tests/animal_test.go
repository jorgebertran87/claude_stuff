package domain_test

import (
	"testing"

	"claude_tdd/domain"
)

func TestHungryAnimalEatsFood(t *testing.T) {
	tests := []struct {
		species         domain.Species
		food            string
		expectedEmotion domain.Emotion
		expectedEnergy  int
	}{
		{domain.Dog, "bone", domain.EmotionOk, 2},
		{domain.Cat, "fish", domain.EmotionSatisfied, 5},
		{domain.Duck, "shit", domain.EmotionSad, -10},
	}

	for _, tt := range tests {
		t.Run(string(tt.species)+" eating "+tt.food, func(t *testing.T) {
			animal := domain.NewAnimal(tt.species)

			if animal.State() != "hungry" {
				t.Errorf("expected state 'hungry', got '%s'", animal.State())
			}

			animal.Feed(tt.food)

			if animal.Emotion() == nil {
				t.Fatal("expected emotion to be set after feeding")
			}
			if *animal.Emotion() != tt.expectedEmotion {
				t.Errorf("expected emotion '%s', got '%s'", tt.expectedEmotion, *animal.Emotion())
			}
			if animal.Energy() != tt.expectedEnergy {
				t.Errorf("expected energy %d, got %d", tt.expectedEnergy, animal.Energy())
			}
		})
	}
}

func TestDifferentAnimalsMakeDifferentSounds(t *testing.T) {
	tests := []struct {
		species       domain.Species
		expectedSound string
	}{
		{domain.Dog, "Woof"},
		{domain.Cat, "Meow"},
		{domain.Duck, "Quack"},
	}

	for _, tt := range tests {
		t.Run(string(tt.species)+" should make a "+tt.expectedSound+" sound", func(t *testing.T) {
			animal := domain.NewAnimal(tt.species)

			sound := animal.MakeSound()

			if sound != tt.expectedSound {
				t.Errorf("expected sound '%s', got '%s'", tt.expectedSound, sound)
			}
		})
	}
}
