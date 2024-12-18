#![allow(unused)]

use std::collections::{HashMap, HashSet};

// common data structures

#[derive(Debug)]
pub struct Task {
    pub id: String,
    pub prio: u8,
    pub deadline: u32,
    pub inter_arrival: u32,
    pub trace: Trace,
}

//#[derive(Debug, Clone)]
#[derive(Debug)]
pub struct Trace {
    pub id: String,
    pub start: u32,
    pub end: u32,
    pub inner: Vec<Trace>,
}

// useful types

// Our task set
pub type Tasks = Vec<Task>;

// A map from Task/Resource identifiers to priority
pub type IdPrio = HashMap<String, u8>;

// A map from Task identifiers to a set of Resource identifiers
pub type TaskResources = HashMap<String, HashSet<String>>;

// Derives the above maps from a set of tasks
pub fn pre_analysis(tasks: &Tasks) -> (IdPrio, TaskResources) {
    let mut ip: IdPrio = HashMap::new();
    let mut tr: TaskResources = HashMap::new();
    for t in tasks {
        update_prio(t.prio, &t.trace, &mut ip);
        for i in &t.trace.inner {
            update_tr(t.id.clone(), i, &mut tr);
        }
    }
    (ip, tr)
}

// helper functions
fn update_prio(prio: u8, trace: &Trace, hm: &mut IdPrio) {
    if let Some(old_prio) = hm.get(&trace.id) {
        if prio > *old_prio {
            hm.insert(trace.id.clone(), prio);
        }
    } else {
        hm.insert(trace.id.clone(), prio);
    }
    for cs in &trace.inner {
        update_prio(prio, cs, hm);
    }
}

fn update_tr(s: String, trace: &Trace, trmap: &mut TaskResources) {
    if let Some(seen) = trmap.get_mut(&s) {
        seen.insert(trace.id.clone());
    } else {
        let mut hs = HashSet::new();
        hs.insert(trace.id.clone());
        trmap.insert(s.clone(), hs);
    }
    for trace in &trace.inner {
        update_tr(s.clone(), trace, trmap);
    }
}
pub enum PreemptionMode {
    Exact,
    Approximate,
}

pub trait Schedulable {
    fn wcet(&self) -> u32;
    fn resources<'a>(&'a self) -> Box<dyn Iterator<Item = &Trace> + 'a>;
}

pub trait TaskSchedulable: Schedulable {
    fn blocking_time<T>(&self, tasks: &T) -> u32
    where
        T: std::ops::Deref<Target = [Task]> + Sized;
    fn busy_period<T>(&self, tasks: &T) -> u32
    where
        T: std::ops::Deref<Target = [Task]> + Sized;
    fn interference<T>(&self, tasks: &T) -> u32
    where
        T: std::ops::Deref<Target = [Task]> + Sized;
    fn response_time<T>(&self, tasks: &T, mode: &PreemptionMode) -> Result<u32, String>
    where
        T: std::ops::Deref<Target = [Task]> + Sized;
}

pub trait TraceSchedulable: Schedulable {
    fn ceiling_priority<T>(&self, tasks: &T) -> u8
    where
        T: std::ops::Deref<Target = [Task]> + Sized;
}

impl Schedulable for Task {
    /// C(t)
    #[inline(always)]
    fn wcet(&self) -> u32 {
        self.trace.end - self.trace.start
    }
    /// Creates an dynamic iterator of all resources within this task recursively.
    fn resources<'a>(&'a self) -> Box<dyn Iterator<Item = &Trace> + 'a> {
        self.trace.resources()
    }
}

impl Schedulable for Trace {
    /// C(t)
    #[inline(always)]
    fn wcet(&self) -> u32 {
        self.end - self.start
    }
    /// Creates an dynamic iterator of all resources within this trace recursively.
    fn resources<'a>(&'a self) -> Box<dyn Iterator<Item = &Trace> + 'a> {
        Box::new(
        self.inner.iter()
            .chain(self.inner.iter().flat_map(|n| n.resources())),
        )
    }
}

impl TaskSchedulable for Task {
    /// B(t) = max(C(l_r)) where P(l) < P(t) and π(l_r) >= P(t)
    fn blocking_time<T>(&self, tasks: &T) -> u32
    where
        T: std::ops::Deref<Target = [Task]> + Sized
    {
        // Firstly, filter tasks by priority, only including lower priority tasks
        // Secondly, for that task, filter all of its associated resources if their ceiling priorities are larger than the target's task.
        // Lastly, with an iterator of P(l) < P(t) and π(l_r) >= P(t), take out the max of those blockings.
        tasks.iter()
            .filter(|l| l.prio < self.prio)
            .flat_map(|lower_priority_task| {
                lower_priority_task.resources().filter_map(|resource| {

                    // Critical section duration of a resource
                    let critical_section = resource.wcet();

                    // Compute ceiling priority of the specified resource within all tasks in &[Task]
                    let ceiling_priority = resource.ceiling_priority(tasks);

                    // Check if the ceiling priority satisfies the condition
                    if ceiling_priority >= self.prio {
                        Some(critical_section)
                    } else {
                        None
                    }
                })
            })
            .max() // Find the maximum critical section time
            .unwrap_or(0) // Return 0 if no valid critical section found
    }

    /// Bp(t)
    fn busy_period<T>(&self, tasks: &T) -> u32
    where
        T: std::ops::Deref<Target = [Task]> + Sized
    {
        tasks.iter()
            .filter(|t| t.prio >= self.prio)
            .map(|t| t.wcet())
            .sum()
    }

    /// I(t) = sum(C(h) * ceiling(Bp(t) / A(h))) for all tasks h where P(h) > P(t)
    fn interference<T>(&self, tasks: &T) -> u32
    where
        T: std::ops::Deref<Target = [Task]> + Sized
    {
        tasks.iter()
            .filter(|h| h.prio > self.prio)
            .map(|h| h.wcet() * (self.busy_period(tasks) as f32 / h.inter_arrival as f32).ceil() as u32)
            .sum()
    }

    /// R(t) = B(t) + C(t) + I(t)
    fn response_time<T>(&self, tasks: &T, mode: &PreemptionMode) -> Result<u32, String>
    where
        T: std::ops::Deref<Target = [Task]> + Sized
    {
        let b_t = self.blocking_time(tasks);  // Compute blocking time
        let c_t = self.wcet();                 // Compute critical time
        match mode {
            PreemptionMode::Approximate => {
                let i_t = self.interference(tasks);   // Compute interference
                let response_time = b_t + c_t + i_t;    // R(t) = B(t) + C(t) + I(t)
                Ok(response_time)
            },
            PreemptionMode::Exact => {
                let mut total_response_time = b_t + c_t;

                // Recursively calculate interference from higher-priority tasks
                for higher_priority_task in tasks.iter().filter(|h| h.prio > self.prio) {
                    let interference = higher_priority_task.response_time(tasks, mode)?;
                    
                    total_response_time += interference;
                }

                // Check against the deadline
                if total_response_time > self.deadline {
                    Err("Deadline missed".to_string())
                } else {
                    Ok(total_response_time)
                }
            },
        }
    }
}

impl TraceSchedulable for Trace {
    /// Calculate ceiling priority π(r) of a given resource as a &Trace, against a set of tasks potentially using the given resource.
    fn ceiling_priority<T>(&self, tasks: &T) -> u8
    where
        T: std::ops::Deref<Target = [Task]> + Sized
    {
        // Iterate through the entire task set, matching any resources id corresponding
        // with the given resource. The set of task matches is transformed into their priorities, then return max value.
        tasks.iter()
            .filter(|task| task.resources().any(|res| res.id == self.id))
            .map(|task| task.prio)
            .max()
            .unwrap_or(1)
    }
}
/// L_tot = sum(L(T)) where L(t) = C(t) / A(t) for all t in &Tasks.
pub fn total_load_factor<T>(tasks: &T) -> Result<f32, String>
where
    // .iter() returns a slice iterator, so the generic T needs to be able to dereference into a slice.
    // We cannot use .into_iter() because it consumes the collection, making it non-existent afterwards.
    T: std::ops::Deref<Target = [Task]> + Sized
{
    tasks.iter()
        .try_fold(0.0, |acc, task| {
            // Check for division by zero
            if task.inter_arrival == 0 {
                Err(format!("Error: Task '{}' has an inter_arrival time of zero.", task.id))
            } else {
                // Calculate the load factor and accumulate the result
                Ok(acc + task.wcet() as f32 / task.inter_arrival as f32)
            }
        })
}

/// Performs the stack resource policy analysis on the given task-set and return results in a formatted Vec<>:
/// 
/// Vec<&Task, R(t), C(t), B(t), I(t)>
pub fn srp_analyze<'a, T>(tasks: &'a T, mode: &PreemptionMode) -> Vec<(&'a Task, Result<u32, String>, u32, u32, u32)> 
where
    T: std::ops::Deref<Target = [Task]> + Sized
{
    let mut result_vector = Vec::new();

    for task in tasks.iter() {
        
        let response_time = task.response_time(tasks, mode);
        let blocking_time = task.blocking_time(tasks);
        let critical_time = task.wcet();
        let interference = task.interference(tasks);

        result_vector.push((task, response_time, blocking_time, critical_time, interference));
    }

    result_vector
}
