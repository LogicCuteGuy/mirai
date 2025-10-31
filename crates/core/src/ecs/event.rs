//! Event system for inter-system communication

use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;

/// Trait marker for types that can be used as events
pub trait Event: Send + Sync + 'static {}

/// Event priority for ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    Lowest = 0,
    Low = 1,
    Normal = 2,
    High = 3,
    Highest = 4,
}

impl Default for EventPriority {
    fn default() -> Self {
        EventPriority::Normal
    }
}

/// Event wrapper with metadata
#[derive(Debug)]
pub struct EventWrapper {
    event: Box<dyn Any + Send + Sync>,
    priority: EventPriority,
    #[allow(dead_code)]
    timestamp: std::time::Instant,
}

impl EventWrapper {
    fn new<T: Event>(event: T, priority: EventPriority) -> Self {
        Self {
            event: Box::new(event),
            priority,
            timestamp: std::time::Instant::now(),
        }
    }
    
    fn downcast<T: Event>(self) -> Option<T> {
        self.event.downcast::<T>().ok().map(|boxed| *boxed)
    }
    
    pub fn downcast_ref<T: Event>(&self) -> Option<&T> {
        self.event.downcast_ref::<T>()
    }
}

/// Event bus for managing event distribution
pub struct EventBus {
    events: HashMap<TypeId, VecDeque<EventWrapper>>,
    subscribers: HashMap<TypeId, Vec<Box<dyn EventHandler>>>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            subscribers: HashMap::new(),
        }
    }
    
    /// Send an event with default priority
    pub fn send<T: Event>(&mut self, event: T) {
        self.send_with_priority(event, EventPriority::default());
    }
    
    /// Send an event with specific priority
    pub fn send_with_priority<T: Event>(&mut self, event: T, priority: EventPriority) {
        let type_id = TypeId::of::<T>();
        let wrapper = EventWrapper::new(event, priority);
        
        let queue = self.events.entry(type_id).or_insert_with(VecDeque::new);
        
        // Insert in priority order (highest priority first)
        let insert_pos = queue.iter().position(|existing| existing.priority < priority)
            .unwrap_or(queue.len());
        
        queue.insert(insert_pos, wrapper);
    }
    
    /// Subscribe to events of a specific type
    pub fn subscribe<T: Event, H: EventHandler + 'static>(&mut self, handler: H) {
        let type_id = TypeId::of::<T>();
        self.subscribers.entry(type_id).or_insert_with(Vec::new).push(Box::new(handler));
    }
    
    /// Process all pending events
    pub fn process_events(&mut self) {
        let mut events_to_process = HashMap::new();
        
        // Collect all events
        for (type_id, queue) in &mut self.events {
            if !queue.is_empty() {
                events_to_process.insert(*type_id, std::mem::take(queue));
            }
        }
        
        // Process events by type
        for (type_id, mut queue) in events_to_process {
            if let Some(handlers) = self.subscribers.get_mut(&type_id) {
                while let Some(event_wrapper) = queue.pop_front() {
                    for handler in handlers.iter_mut() {
                        handler.handle_event(&event_wrapper);
                    }
                }
            }
        }
    }
    
    /// Get events of a specific type without consuming them
    pub fn read_events<T: Event>(&self) -> EventReader<'_, T> {
        let type_id = TypeId::of::<T>();
        let events = self.events.get(&type_id)
            .map(|queue| queue.iter().collect())
            .unwrap_or_default();
        
        EventReader::new(events)
    }
    
    /// Consume events of a specific type
    pub fn drain_events<T: Event>(&mut self) -> EventDrainer<T> {
        let type_id = TypeId::of::<T>();
        let events = self.events.remove(&type_id)
            .unwrap_or_default();
        
        EventDrainer::new(events)
    }
    
    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
    }
    
    /// Get the number of pending events of a specific type
    pub fn event_count<T: Event>(&self) -> usize {
        let type_id = TypeId::of::<T>();
        self.events.get(&type_id).map(|queue| queue.len()).unwrap_or(0)
    }
    
    /// Check if there are any pending events of a specific type
    pub fn has_events<T: Event>(&self) -> bool {
        self.event_count::<T>() > 0
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for handling events
pub trait EventHandler: Send + Sync {
    fn handle_event(&mut self, event: &EventWrapper);
}

/// Function-based event handler
pub struct FunctionEventHandler<T: Event> {
    handler: Box<dyn FnMut(&T) + Send + Sync>,
    _phantom: PhantomData<T>,
}

impl<T: Event> FunctionEventHandler<T> {
    pub fn new<F>(handler: F) -> Self
    where
        F: FnMut(&T) + Send + Sync + 'static,
    {
        Self {
            handler: Box::new(handler),
            _phantom: PhantomData,
        }
    }
}

impl<T: Event> EventHandler for FunctionEventHandler<T> {
    fn handle_event(&mut self, event: &EventWrapper) {
        if let Some(typed_event) = event.downcast_ref::<T>() {
            (self.handler)(typed_event);
        }
    }
}

/// Reader for events without consuming them
pub struct EventReader<'a, T> {
    events: Vec<&'a EventWrapper>,
    current: usize,
    _phantom: PhantomData<T>,
}

impl<'a, T: Event> EventReader<'a, T> {
    fn new(events: Vec<&'a EventWrapper>) -> Self {
        Self {
            events,
            current: 0,
            _phantom: PhantomData,
        }
    }
    
    /// Get the number of events
    pub fn len(&self) -> usize {
        self.events.len()
    }
    
    /// Check if there are no events
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl<'a, T: Event> Iterator for EventReader<'a, T> {
    type Item = &'a T;
    
    fn next(&mut self) -> Option<Self::Item> {
        while self.current < self.events.len() {
            let event_wrapper = self.events[self.current];
            self.current += 1;
            
            if let Some(event) = event_wrapper.downcast_ref::<T>() {
                return Some(event);
            }
        }
        None
    }
}

/// Drainer for consuming events
pub struct EventDrainer<T> {
    events: VecDeque<EventWrapper>,
    _phantom: PhantomData<T>,
}

impl<T: Event> EventDrainer<T> {
    fn new(events: VecDeque<EventWrapper>) -> Self {
        Self {
            events,
            _phantom: PhantomData,
        }
    }
    
    /// Get the number of events
    pub fn len(&self) -> usize {
        self.events.len()
    }
    
    /// Check if there are no events
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

impl<T: Event> Iterator for EventDrainer<T> {
    type Item = T;
    
    fn next(&mut self) -> Option<Self::Item> {
        while let Some(event_wrapper) = self.events.pop_front() {
            if let Some(event) = event_wrapper.downcast::<T>() {
                return Some(event);
            }
        }
        None
    }
}