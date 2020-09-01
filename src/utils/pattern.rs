use std::fmt;

use num_integer::Roots;

pub enum Pattern {
    Line(usize),
    Rectangle(usize, usize),
    Triangle(usize),
    Diamond(usize, usize),
}

impl Pattern {
    pub fn new(count: usize) -> Self {
        //override for the smaller ones
        if count < 5 {
            return Pattern::Line(count);
        }

        // check if we can make a triangle
        if let Some(base) = triangular_base(count) {
            return Pattern::Triangle(base);
        }

        // a diamond is just 2 triangles!
        // or even more simple: a rotated square!
        let width = count.sqrt();
        if width.pow(2) == count {
            return Pattern::Diamond(width, 2 * width - 1);
        }

        // check if we can make rectangles with these ratios
        for ratio in 2..5 {
            let height = (count / ratio).sqrt();
            let width = ratio * height;
            //do we have a full rectangle?
            if width * height == count {
                println!("count: {} ({} * {})", width * height, width, height);
                return Pattern::Rectangle(width, height);
            }
        }
        // for low counts a line is acceptable
        if count <= 10 {
            Pattern::Line(count)
        } else {
            // go with a in complete 3 ratio rectangle
            let height = ((count as f32) / 3.0).sqrt().ceil() as usize;
            let width = 3 * height;
            return Pattern::Rectangle(width, height);
        }
    }

    pub fn arrange<T>(&self, mut list: Vec<T>) -> Vec<Vec<T>> {
        if list.is_empty() {
            return vec![vec![]];
        }

        match self {
            Pattern::Line(length) => {
                let mut out = Vec::with_capacity(*length as usize);
                for item in list.drain(0..*length as usize) {
                    out.push(item)
                }
                vec![out]
            }
            Pattern::Rectangle(width, height) => {
                let mut out = Vec::with_capacity(*height as usize);
                for _ in 0..*height as usize {
                    let mut row = Vec::with_capacity(*width as usize);
                    for _ in 0..*width as usize {
                        if list.is_empty() {
                            break;
                        }
                        row.push(list.remove(0));
                    }
                    out.push(row)
                }
                out
            }
            Pattern::Triangle(size) => triangular_rows(&mut list, *size, *size, true),
            Pattern::Diamond(width, height) => {
                let mut out = Vec::with_capacity(*height as usize);
                out.append(&mut triangular_rows(&mut list, *width, *width, true));
                out.append(&mut triangular_rows(&mut list, *width - 1, *width - 1, false));
                out
            }
        }
    }
}

fn triangular_rows<T>(input: &mut Vec<T>, width: usize, height: usize, increment: bool) -> Vec<Vec<T>> {
    let mut out = Vec::with_capacity(height as usize);
    let mut offset = if increment { width - 1 } else { 0 };
    for _ in 0..height as usize {
        let mut row = Vec::with_capacity((width - offset) as usize);
        for _ in 0..(width - offset) {
            row.push(input.remove(0));
        }
        out.push(row);
        if out.is_empty() {
            break;
        }
        if increment && offset > 0 {
            offset -= 1
        } else {
            offset += 1;
        }
    }
    out
}
fn triangular_base(count: usize) -> Option<usize> {
    // formula for determining the size of a triangle for a triangular number:
    // m = (sqrt(8n+1) - 1) / 2
    // thus we can fit it in a triangle with no holes if and only if
    // 8n+1 is a perfect square root
    //TODO: find better names
    let big_number = (8 * count) + 1;
    let smaller_number = big_number.sqrt();
    if big_number == smaller_number.pow(2) {
        return Some((smaller_number - 1) / 2);
    } else {
        None
    }
}

/// *
/// ** //3
/// *** //6
/// **** //10
/// ***** //15
/// ****** // 21
/// ***** // 26
/// **** // 30
/// *** //33
/// ** // 35
/// * //36
/// 26

///
/// *
/// **
/// ***
/// ****
/// *****
/// ******
/// 21

/// s = w * h | w = 2h
/// s = 2h * h
/// s/2 = h²
/// sqrt(s/2) = h

/// s = (n*(n+1))/2 + ((n-1)*(n))/2
/// 2s = (n*(n+1) + (n-1) * n
/// 2s = n² + n + n² -n
/// 2s = 2n²
/// s = n²
///

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Pattern::Line(length) => write!(f, "Line ({})", length.to_string()),
            Pattern::Rectangle(width, height) => write!(f, "Rectangle ({}, {})", width.to_string(), height.to_string()),
            Pattern::Triangle(size) => write!(f, "Triangle ({})", size.to_string()),
            Pattern::Diamond(width, height) => write!(f, "Diamond ({}, {})", width.to_string(), height.to_string()),
        }
    }
}
